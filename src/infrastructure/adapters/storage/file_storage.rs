//! File Storage - 文件系统音频存储实现
//!
//! 实现 AudioStoragePort trait

use async_trait::async_trait;
use std::path::{Path, PathBuf};
use tokio::fs;
use uuid::Uuid;

use crate::application::ports::{
    AudioStorageError, AudioStoragePort, GcConfig, GcResult, StorageStats,
};

/// 文件系统音频存储
pub struct FileAudioStorage {
    /// 存储根目录
    base_dir: PathBuf,
}

impl FileAudioStorage {
    /// 创建新的文件存储
    pub async fn new(base_dir: impl AsRef<Path>) -> Result<Self, AudioStorageError> {
        let base_dir = base_dir.as_ref().to_path_buf();

        // 确保目录存在
        fs::create_dir_all(&base_dir)
            .await
            .map_err(|e| AudioStorageError::IoError(e.to_string()))?;

        Ok(Self { base_dir })
    }

    /// 获取存储根目录
    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }
}

#[async_trait]
impl AudioStoragePort for FileAudioStorage {
    fn get_session_dir(&self, session_id: Uuid) -> PathBuf {
        self.base_dir.join(session_id.to_string())
    }

    fn get_audio_path(&self, session_id: Uuid, segment_index: usize) -> PathBuf {
        self.get_session_dir(session_id)
            .join(format!("segment_{}.wav", segment_index))
    }

    async fn save_audio(
        &self,
        session_id: Uuid,
        segment_index: usize,
        data: &[u8],
    ) -> Result<PathBuf, AudioStorageError> {
        let session_dir = self.get_session_dir(session_id);

        // 确保会话目录存在
        fs::create_dir_all(&session_dir)
            .await
            .map_err(|e| AudioStorageError::IoError(e.to_string()))?;

        let audio_path = self.get_audio_path(session_id, segment_index);

        fs::write(&audio_path, data)
            .await
            .map_err(|e| AudioStorageError::IoError(e.to_string()))?;

        tracing::debug!(
            "Saved audio: session={}, segment={}, size={} bytes",
            session_id,
            segment_index,
            data.len()
        );

        Ok(audio_path)
    }

    async fn read_audio(
        &self,
        session_id: Uuid,
        segment_index: usize,
    ) -> Result<Vec<u8>, AudioStorageError> {
        let audio_path = self.get_audio_path(session_id, segment_index);

        if !audio_path.exists() {
            return Err(AudioStorageError::FileNotFound(
                audio_path.to_string_lossy().to_string(),
            ));
        }

        fs::read(&audio_path)
            .await
            .map_err(|e| AudioStorageError::IoError(e.to_string()))
    }

    async fn delete_audio(
        &self,
        session_id: Uuid,
        segment_index: usize,
    ) -> Result<(), AudioStorageError> {
        let audio_path = self.get_audio_path(session_id, segment_index);

        if audio_path.exists() {
            fs::remove_file(&audio_path)
                .await
                .map_err(|e| AudioStorageError::IoError(e.to_string()))?;

            tracing::debug!(
                "Deleted audio: session={}, segment={}",
                session_id,
                segment_index
            );
        }

        Ok(())
    }

    async fn delete_session_audio(&self, session_id: Uuid) -> Result<u64, AudioStorageError> {
        let session_dir = self.get_session_dir(session_id);

        if !session_dir.exists() {
            return Ok(0);
        }

        let mut deleted_count = 0u64;
        let mut entries = fs::read_dir(&session_dir)
            .await
            .map_err(|e| AudioStorageError::IoError(e.to_string()))?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| AudioStorageError::IoError(e.to_string()))?
        {
            if entry.path().extension().map_or(false, |ext| ext == "wav") {
                fs::remove_file(entry.path())
                    .await
                    .map_err(|e| AudioStorageError::IoError(e.to_string()))?;
                deleted_count += 1;
            }
        }

        // 尝试删除空目录
        let _ = fs::remove_dir(&session_dir).await;

        tracing::info!(
            "Deleted session audio: session={}, files={}",
            session_id,
            deleted_count
        );

        Ok(deleted_count)
    }

    async fn audio_exists(&self, session_id: Uuid, segment_index: usize) -> bool {
        self.get_audio_path(session_id, segment_index).exists()
    }

    async fn get_stats(&self) -> Result<StorageStats, AudioStorageError> {
        let mut stats = StorageStats::default();

        let mut entries = fs::read_dir(&self.base_dir)
            .await
            .map_err(|e| AudioStorageError::IoError(e.to_string()))?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| AudioStorageError::IoError(e.to_string()))?
        {
            let path = entry.path();
            if path.is_dir() {
                stats.session_count += 1;

                // 统计该会话下的文件
                if let Ok(mut session_entries) = fs::read_dir(&path).await {
                    while let Ok(Some(file_entry)) = session_entries.next_entry().await {
                        if file_entry
                            .path()
                            .extension()
                            .map_or(false, |ext| ext == "wav")
                        {
                            stats.file_count += 1;
                            if let Ok(metadata) = file_entry.metadata().await {
                                stats.used_bytes += metadata.len();
                            }
                        }
                    }
                }
            }
        }

        Ok(stats)
    }

    async fn gc(&self, _config: &GcConfig) -> Result<GcResult, AudioStorageError> {
        // GC 逻辑需要配合 Repository 使用
        // 这里只是基础实现，实际 GC 由 GcService 协调
        Ok(GcResult::default())
    }

    async fn evict_to_size(&self, target_bytes: u64) -> Result<GcResult, AudioStorageError> {
        let stats = self.get_stats().await?;

        if stats.used_bytes <= target_bytes {
            return Ok(GcResult::default());
        }

        // LRU 清理需要配合 Repository 的 last_accessed_at 信息
        // 这里只是基础框架
        tracing::warn!(
            "Storage exceeds limit: used={} bytes, target={} bytes",
            stats.used_bytes,
            target_bytes
        );

        Ok(GcResult::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_save_and_read_audio() {
        let temp_dir = tempdir().unwrap();
        let storage = FileAudioStorage::new(temp_dir.path()).await.unwrap();

        let session_id = Uuid::new_v4();
        let segment_index = 0;
        let data = b"fake wav data";

        // Save
        let path = storage
            .save_audio(session_id, segment_index, data)
            .await
            .unwrap();
        assert!(path.exists());

        // Read
        let read_data = storage.read_audio(session_id, segment_index).await.unwrap();
        assert_eq!(read_data, data);

        // Exists
        assert!(storage.audio_exists(session_id, segment_index).await);

        // Delete
        storage
            .delete_audio(session_id, segment_index)
            .await
            .unwrap();
        assert!(!storage.audio_exists(session_id, segment_index).await);
    }

    #[tokio::test]
    async fn test_delete_session_audio() {
        let temp_dir = tempdir().unwrap();
        let storage = FileAudioStorage::new(temp_dir.path()).await.unwrap();

        let session_id = Uuid::new_v4();

        // Save multiple segments
        for i in 0..3 {
            storage
                .save_audio(session_id, i, b"data")
                .await
                .unwrap();
        }

        // Delete all
        let deleted = storage.delete_session_audio(session_id).await.unwrap();
        assert_eq!(deleted, 3);

        // Verify deleted
        for i in 0..3 {
            assert!(!storage.audio_exists(session_id, i).await);
        }
    }
}
