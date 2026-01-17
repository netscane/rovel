//! Sled-based LRU Audio Cache Implementation

use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sled::Db;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use uuid::Uuid;

use crate::application::ports::{
    AudioCachePort, CacheError, CacheMetadata, CacheStats,
};

/// Sled 缓存配置
#[derive(Debug, Clone)]
pub struct SledCacheConfig {
    /// 数据库路径
    pub db_path: String,
    /// 最大缓存大小（字节）
    pub max_size_bytes: u64,
}

impl Default for SledCacheConfig {
    fn default() -> Self {
        Self {
            db_path: "data/cache.sled".to_string(),
            max_size_bytes: 10 * 1024 * 1024 * 1024, // 10GB
        }
    }
}

/// 内部缓存条目
#[derive(Debug, Clone, Serialize, Deserialize)]
struct InternalCacheEntry {
    audio_data: Vec<u8>,
    size_bytes: u64,
    duration_ms: u64,
    content_hash: String,
    novel_id: String,
    segment_index: u32,
    voice_id: String,
    last_accessed: i64,
    created_at: i64,
    sample_rate: Option<u32>,
}

/// Sled 音频缓存
pub struct SledAudioCache {
    db: Db,
    max_size_bytes: u64,
    current_size: AtomicU64,
    hit_count: AtomicU64,
    miss_count: AtomicU64,
}

impl SledAudioCache {
    /// 创建新的缓存实例
    pub fn new(config: &SledCacheConfig) -> Result<Self, CacheError> {
        let db = sled::open(&config.db_path)
            .map_err(|e| CacheError::DatabaseError(e.to_string()))?;

        // 计算当前缓存大小
        let current_size = Self::calculate_total_size(&db)?;

        tracing::info!(
            db_path = %config.db_path,
            max_size_bytes = config.max_size_bytes,
            current_size = current_size,
            "SledAudioCache initialized"
        );

        Ok(Self {
            db,
            max_size_bytes: config.max_size_bytes,
            current_size: AtomicU64::new(current_size),
            hit_count: AtomicU64::new(0),
            miss_count: AtomicU64::new(0),
        })
    }

    /// 打开现有缓存
    pub fn open<P: AsRef<Path>>(path: P, max_size_bytes: u64) -> Result<Self, CacheError> {
        let config = SledCacheConfig {
            db_path: path.as_ref().to_string_lossy().to_string(),
            max_size_bytes,
        };
        Self::new(&config)
    }

    pub fn arc(self) -> Arc<Self> {
        Arc::new(self)
    }

    /// 计算数据库中所有条目的总大小
    fn calculate_total_size(db: &Db) -> Result<u64, CacheError> {
        let mut total = 0u64;
        for item in db.scan_prefix("cache:") {
            let (_, value) = item.map_err(|e| CacheError::DatabaseError(e.to_string()))?;
            if let Ok(entry) = bincode::deserialize::<InternalCacheEntry>(&value) {
                total += entry.size_bytes;
            }
        }
        Ok(total)
    }

    /// LRU 淘汰
    fn evict_lru(&self) -> Result<(), CacheError> {
        let mut oldest: Option<(String, InternalCacheEntry)> = None;

        for item in self.db.scan_prefix("cache:") {
            let (key, value) = item.map_err(|e| CacheError::DatabaseError(e.to_string()))?;
            if let Ok(entry) = bincode::deserialize::<InternalCacheEntry>(&value) {
                let is_older = oldest
                    .as_ref()
                    .map(|(_, e)| entry.last_accessed < e.last_accessed)
                    .unwrap_or(true);

                if is_older {
                    let key_str = String::from_utf8(key.to_vec())
                        .map_err(|e| CacheError::SerializationError(e.to_string()))?;
                    oldest = Some((key_str, entry));
                }
            }
        }

        if let Some((key, entry)) = oldest {
            // 删除缓存条目
            self.db
                .remove(&key)
                .map_err(|e| CacheError::DatabaseError(e.to_string()))?;

            // 删除映射
            let mapping_key = format!(
                "mapping:{}:{}:{}",
                entry.novel_id, entry.segment_index, entry.voice_id
            );
            let _ = self.db.remove(&mapping_key);

            self.current_size.fetch_sub(entry.size_bytes, Ordering::Relaxed);
            tracing::debug!(
                key = %key,
                size_bytes = entry.size_bytes,
                "LRU evicted cache entry"
            );
        }

        Ok(())
    }

    /// 刷新数据库
    pub fn flush(&self) -> Result<(), CacheError> {
        self.db
            .flush()
            .map_err(|e| CacheError::DatabaseError(e.to_string()))?;
        Ok(())
    }
}

#[async_trait]
impl AudioCachePort for SledAudioCache {
    async fn put(
        &self,
        cache_key: &str,
        audio_data: Vec<u8>,
        metadata: CacheMetadata,
    ) -> Result<(), CacheError> {
        let size = audio_data.len() as u64;

        // 淘汰以腾出空间
        while self.current_size.load(Ordering::Relaxed) + size > self.max_size_bytes {
            self.evict_lru()?;
        }

        let entry = InternalCacheEntry {
            audio_data,
            size_bytes: size,
            duration_ms: metadata.duration_ms,
            content_hash: metadata.content_hash,
            novel_id: metadata.novel_id.to_string(),
            segment_index: metadata.segment_index,
            voice_id: metadata.voice_id.to_string(),
            last_accessed: Utc::now().timestamp(),
            created_at: Utc::now().timestamp(),
            sample_rate: metadata.sample_rate,
        };

        let entry_bytes =
            bincode::serialize(&entry).map_err(|e| CacheError::SerializationError(e.to_string()))?;

        // 存储缓存条目
        self.db
            .insert(format!("cache:{}", cache_key), entry_bytes)
            .map_err(|e| CacheError::DatabaseError(e.to_string()))?;

        // 存储映射
        let mapping_key = format!(
            "mapping:{}:{}:{}",
            metadata.novel_id, metadata.segment_index, metadata.voice_id
        );
        self.db
            .insert(mapping_key, cache_key.as_bytes())
            .map_err(|e| CacheError::DatabaseError(e.to_string()))?;

        self.current_size.fetch_add(size, Ordering::Relaxed);

        tracing::debug!(
            cache_key = %cache_key,
            size_bytes = size,
            "Audio cached"
        );

        Ok(())
    }

    async fn get(&self, cache_key: &str) -> Result<Option<Vec<u8>>, CacheError> {
        let key = format!("cache:{}", cache_key);

        match self.db.get(&key) {
            Ok(Some(data)) => {
                let mut entry: InternalCacheEntry = bincode::deserialize(&data)
                    .map_err(|e| CacheError::SerializationError(e.to_string()))?;

                // 更新 last_accessed (LRU touch)
                entry.last_accessed = Utc::now().timestamp();
                let entry_bytes = bincode::serialize(&entry)
                    .map_err(|e| CacheError::SerializationError(e.to_string()))?;
                self.db
                    .insert(&key, entry_bytes)
                    .map_err(|e| CacheError::DatabaseError(e.to_string()))?;

                self.hit_count.fetch_add(1, Ordering::Relaxed);
                Ok(Some(entry.audio_data))
            }
            Ok(None) => {
                self.miss_count.fetch_add(1, Ordering::Relaxed);
                Ok(None)
            }
            Err(e) => Err(CacheError::DatabaseError(e.to_string())),
        }
    }

    async fn lookup(
        &self,
        novel_id: Uuid,
        segment_index: u32,
        voice_id: Uuid,
    ) -> Result<Option<String>, CacheError> {
        let mapping_key = format!("mapping:{}:{}:{}", novel_id, segment_index, voice_id);

        match self.db.get(&mapping_key) {
            Ok(Some(data)) => {
                let cache_key = String::from_utf8(data.to_vec())
                    .map_err(|e| CacheError::SerializationError(e.to_string()))?;
                Ok(Some(cache_key))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(CacheError::DatabaseError(e.to_string())),
        }
    }

    async fn exists(&self, cache_key: &str) -> Result<bool, CacheError> {
        let key = format!("cache:{}", cache_key);
        self.db
            .contains_key(&key)
            .map_err(|e| CacheError::DatabaseError(e.to_string()))
    }

    async fn remove(&self, cache_key: &str) -> Result<(), CacheError> {
        let key = format!("cache:{}", cache_key);

        if let Some(data) = self
            .db
            .remove(&key)
            .map_err(|e| CacheError::DatabaseError(e.to_string()))?
        {
            if let Ok(entry) = bincode::deserialize::<InternalCacheEntry>(&data) {
                // 删除映射
                let mapping_key = format!(
                    "mapping:{}:{}:{}",
                    entry.novel_id, entry.segment_index, entry.voice_id
                );
                let _ = self.db.remove(&mapping_key);

                self.current_size.fetch_sub(entry.size_bytes, Ordering::Relaxed);
            }
        }

        Ok(())
    }

    async fn stats(&self) -> CacheStats {
        let total_entries = self.db.scan_prefix("cache:").count();

        CacheStats {
            total_entries,
            total_size_bytes: self.current_size.load(Ordering::Relaxed),
            max_size_bytes: self.max_size_bytes,
            hit_count: self.hit_count.load(Ordering::Relaxed),
            miss_count: self.miss_count.load(Ordering::Relaxed),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_cache_put_get() {
        let dir = tempdir().unwrap();
        let config = SledCacheConfig {
            db_path: dir.path().join("test.sled").to_string_lossy().to_string(),
            max_size_bytes: 1024 * 1024,
        };

        let cache = SledAudioCache::new(&config).unwrap();

        let audio_data = vec![1, 2, 3, 4, 5];
        let metadata = CacheMetadata {
            novel_id: Uuid::new_v4(),
            segment_index: 0,
            voice_id: Uuid::new_v4(),
            content_hash: "test_hash".to_string(),
            duration_ms: 1000,
            sample_rate: Some(22050),
        };

        // Put
        cache.put("test_key", audio_data.clone(), metadata).await.unwrap();

        // Get
        let result = cache.get("test_key").await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap(), audio_data);

        // Exists
        let exists = cache.exists("test_key").await.unwrap();
        assert!(exists);

        // Stats
        let stats = cache.stats().await;
        assert_eq!(stats.total_entries, 1);
        assert_eq!(stats.hit_count, 1);
    }

    #[tokio::test]
    async fn test_cache_lookup() {
        let dir = tempdir().unwrap();
        let config = SledCacheConfig {
            db_path: dir.path().join("test.sled").to_string_lossy().to_string(),
            max_size_bytes: 1024 * 1024,
        };

        let cache = SledAudioCache::new(&config).unwrap();

        let novel_id = Uuid::new_v4();
        let voice_id = Uuid::new_v4();
        let metadata = CacheMetadata {
            novel_id,
            segment_index: 5,
            voice_id,
            content_hash: "test_hash".to_string(),
            duration_ms: 1000,
            sample_rate: Some(22050),
        };

        cache.put("my_cache_key", vec![1, 2, 3], metadata).await.unwrap();

        // Lookup by novel_id + segment_index + voice_id
        let result = cache.lookup(novel_id, 5, voice_id).await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap(), "my_cache_key");
    }
}
