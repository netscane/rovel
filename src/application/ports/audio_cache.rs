//! Audio Cache Port - 音频缓存管理
//!
//! 定义音频缓存的抽象接口，具体实现使用 Sled (LRU 缓存)

use async_trait::async_trait;
use thiserror::Error;
use uuid::Uuid;

/// Audio Cache 错误
#[derive(Debug, Error)]
pub enum CacheError {
    #[error("Cache entry not found: {0}")]
    NotFound(String),

    #[error("Cache full, eviction failed")]
    EvictionFailed,

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("IO error: {0}")]
    IoError(String),

    #[error("Database error: {0}")]
    DatabaseError(String),
}

/// 缓存元数据
#[derive(Debug, Clone)]
pub struct CacheMetadata {
    pub novel_id: Uuid,
    pub segment_index: u32,
    pub voice_id: Uuid,
    pub content_hash: String,
    pub duration_ms: u64,
    pub sample_rate: Option<u32>,
}

/// 缓存条目
#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub audio_data: Vec<u8>,
    pub metadata: CacheMetadata,
    pub size_bytes: u64,
    pub last_accessed: i64,
    pub created_at: i64,
}

/// Audio Cache Port
///
/// 基于 content hash + voice_id 的 LRU 缓存
/// - 缓存 key: md5(segment_content) + voice_id
/// - 支持通过 novel_id + segment_index + voice_id 查找
#[async_trait]
pub trait AudioCachePort: Send + Sync {
    /// 存储音频数据
    ///
    /// 自动执行 LRU 淘汰以保持缓存大小在限制内
    async fn put(
        &self,
        cache_key: &str,
        audio_data: Vec<u8>,
        metadata: CacheMetadata,
    ) -> Result<(), CacheError>;

    /// 根据缓存 key 获取音频数据
    ///
    /// 同时更新 last_accessed 时间戳（LRU touch）
    async fn get(&self, cache_key: &str) -> Result<Option<Vec<u8>>, CacheError>;

    /// 根据 novel_id + segment_index + voice_id 查找缓存 key
    async fn lookup(
        &self,
        novel_id: Uuid,
        segment_index: u32,
        voice_id: Uuid,
    ) -> Result<Option<String>, CacheError>;

    /// 检查缓存是否存在
    async fn exists(&self, cache_key: &str) -> Result<bool, CacheError>;

    /// 删除缓存条目
    async fn remove(&self, cache_key: &str) -> Result<(), CacheError>;

    /// 获取缓存统计信息
    async fn stats(&self) -> CacheStats;
}

/// 缓存统计信息
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    pub total_entries: usize,
    pub total_size_bytes: u64,
    pub max_size_bytes: u64,
    pub hit_count: u64,
    pub miss_count: u64,
}

/// 生成缓存 key
///
/// 使用 md5(segment_content) + voice_id 作为缓存 key
pub fn generate_cache_key(segment_content: &str, voice_id: &Uuid) -> String {
    let digest = md5::compute(segment_content.as_bytes());
    let content_hash = format!("{:x}", digest);
    format!("{}:{}", content_hash, voice_id)
}
