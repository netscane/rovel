//! Audio Storage Port - 出站端口
//!
//! 定义音频文件存储和 GC 的抽象接口

use async_trait::async_trait;
use std::path::PathBuf;
use thiserror::Error;
use uuid::Uuid;

/// 音频存储错误
#[derive(Debug, Error)]
pub enum AudioStorageError {
    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("IO error: {0}")]
    IoError(String),

    #[error("Storage full: used {used} bytes, limit {limit} bytes")]
    StorageFull { used: u64, limit: u64 },
}

/// GC 配置
#[derive(Debug, Clone)]
pub struct GcConfig {
    /// 窗口外音频保留时间（秒），超时清理
    pub window_evict_delay_secs: u64,
    /// Session 过期时间（秒），无访问则清理
    pub session_expire_secs: u64,
    /// 最大存储空间（字节）
    pub max_storage_bytes: u64,
    /// GC 定时任务间隔（秒）
    pub gc_interval_secs: u64,
}

impl Default for GcConfig {
    fn default() -> Self {
        Self {
            window_evict_delay_secs: 300,       // 5 分钟
            session_expire_secs: 86400,         // 24 小时
            max_storage_bytes: 1024 * 1024 * 1024, // 1 GB
            gc_interval_secs: 3600,             // 1 小时
        }
    }
}

/// 存储统计
#[derive(Debug, Clone, Default)]
pub struct StorageStats {
    /// 已使用空间（字节）
    pub used_bytes: u64,
    /// 文件数量
    pub file_count: u64,
    /// 会话数量
    pub session_count: u64,
}

/// GC 结果
#[derive(Debug, Clone, Default)]
pub struct GcResult {
    /// 删除的文件数量
    pub deleted_files: u64,
    /// 释放的空间（字节）
    pub freed_bytes: u64,
    /// 清理的会话数量
    pub cleaned_sessions: u64,
}

/// Audio Storage Port - 出站端口
///
/// 管理音频文件的存储和垃圾回收
#[async_trait]
pub trait AudioStoragePort: Send + Sync {
    /// 获取会话的音频存储目录
    fn get_session_dir(&self, session_id: Uuid) -> PathBuf;

    /// 获取音频文件路径
    fn get_audio_path(&self, session_id: Uuid, segment_index: usize) -> PathBuf;

    /// 保存音频数据
    async fn save_audio(
        &self,
        session_id: Uuid,
        segment_index: usize,
        data: &[u8],
    ) -> Result<PathBuf, AudioStorageError>;

    /// 读取音频数据
    async fn read_audio(
        &self,
        session_id: Uuid,
        segment_index: usize,
    ) -> Result<Vec<u8>, AudioStorageError>;

    /// 删除音频文件
    async fn delete_audio(
        &self,
        session_id: Uuid,
        segment_index: usize,
    ) -> Result<(), AudioStorageError>;

    /// 删除会话的所有音频
    async fn delete_session_audio(&self, session_id: Uuid) -> Result<u64, AudioStorageError>;

    /// 检查音频是否存在
    async fn audio_exists(&self, session_id: Uuid, segment_index: usize) -> bool;

    /// 获取存储统计
    async fn get_stats(&self) -> Result<StorageStats, AudioStorageError>;

    /// 执行垃圾回收
    async fn gc(&self, config: &GcConfig) -> Result<GcResult, AudioStorageError>;

    /// 按 LRU 清理到指定空间
    async fn evict_to_size(&self, target_bytes: u64) -> Result<GcResult, AudioStorageError>;
}
