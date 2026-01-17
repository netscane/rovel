//! Repository Ports - 出站端口
//!
//! 定义数据持久化的抽象接口
//! 具体实现在 infrastructure 层（如 SQLite）

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::path::PathBuf;
use thiserror::Error;
use uuid::Uuid;

/// Repository 错误
#[derive(Debug, Error)]
pub enum RepositoryError {
    #[error("Entity not found: {0}")]
    NotFound(String),

    #[error("Duplicate entity: {0}")]
    Duplicate(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("IO error: {0}")]
    IoError(String),
}

// ============================================================================
// Novel Repository
// ============================================================================

/// 小说处理状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NovelStatus {
    /// 处理中
    Processing,
    /// 已就绪
    Ready,
    /// 处理失败
    Failed,
}

impl NovelStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            NovelStatus::Processing => "processing",
            NovelStatus::Ready => "ready",
            NovelStatus::Failed => "failed",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "processing" => Some(NovelStatus::Processing),
            "ready" => Some(NovelStatus::Ready),
            "failed" => Some(NovelStatus::Failed),
            _ => None,
        }
    }
}

impl Default for NovelStatus {
    fn default() -> Self {
        NovelStatus::Ready
    }
}

/// 小说实体（用于持久化）
#[derive(Debug, Clone)]
pub struct NovelRecord {
    pub id: Uuid,
    pub title: String,
    pub raw_text_path: PathBuf,
    pub total_segments: usize,
    pub status: NovelStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 文本段落实体
#[derive(Debug, Clone)]
pub struct TextSegmentRecord {
    pub id: Uuid,
    pub novel_id: Uuid,
    pub index: usize,
    pub content: String,
    pub char_count: usize,
}

/// Novel Repository Port
#[async_trait]
pub trait NovelRepositoryPort: Send + Sync {
    /// 保存小说
    async fn save(&self, novel: &NovelRecord) -> Result<(), RepositoryError>;

    /// 根据 ID 查找小说
    async fn find_by_id(&self, id: Uuid) -> Result<Option<NovelRecord>, RepositoryError>;

    /// 获取所有小说
    async fn find_all(&self) -> Result<Vec<NovelRecord>, RepositoryError>;

    /// 删除小说
    async fn delete(&self, id: Uuid) -> Result<(), RepositoryError>;

    /// 保存文本段落
    async fn save_segments(&self, segments: &[TextSegmentRecord]) -> Result<(), RepositoryError>;

    /// 获取小说的所有段落
    async fn find_segments_by_novel_id(
        &self,
        novel_id: Uuid,
    ) -> Result<Vec<TextSegmentRecord>, RepositoryError>;

    /// 获取指定段落
    async fn find_segment(
        &self,
        novel_id: Uuid,
        index: usize,
    ) -> Result<Option<TextSegmentRecord>, RepositoryError>;

    /// 分页获取小说段落（性能优化）
    async fn find_segments_paginated(
        &self,
        novel_id: Uuid,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<TextSegmentRecord>, RepositoryError>;

    /// 批量获取指定索引的段落（性能优化）
    async fn find_segments_by_indices(
        &self,
        novel_id: Uuid,
        indices: &[u32],
    ) -> Result<Vec<TextSegmentRecord>, RepositoryError>;

    /// 更新小说状态
    async fn update_status(
        &self,
        id: Uuid,
        status: NovelStatus,
        total_segments: usize,
    ) -> Result<(), RepositoryError>;

    /// 批量保存文本段落（性能优化）
    async fn save_segments_batch(&self, segments: &[TextSegmentRecord]) -> Result<(), RepositoryError> {
        // 默认实现：调用 save_segments
        self.save_segments(segments).await
    }
}

// ============================================================================
// Voice Repository
// ============================================================================

/// 音色实体（用于持久化）
#[derive(Debug, Clone)]
pub struct VoiceRecord {
    pub id: Uuid,
    pub name: String,
    pub reference_audio_path: PathBuf,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Voice Repository Port
#[async_trait]
pub trait VoiceRepositoryPort: Send + Sync {
    /// 保存音色
    async fn save(&self, voice: &VoiceRecord) -> Result<(), RepositoryError>;

    /// 根据 ID 查找音色
    async fn find_by_id(&self, id: Uuid) -> Result<Option<VoiceRecord>, RepositoryError>;

    /// 获取所有音色
    async fn find_all(&self) -> Result<Vec<VoiceRecord>, RepositoryError>;

    /// 删除音色
    async fn delete(&self, id: Uuid) -> Result<(), RepositoryError>;
}

// ============================================================================
// Playback Session Repository
// ============================================================================

/// 播放会话状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    /// 空闲
    Idle,
    /// 播放中
    Playing,
    /// 暂停
    Paused,
    /// 已完成
    Finished,
}

impl SessionState {
    pub fn as_str(&self) -> &'static str {
        match self {
            SessionState::Idle => "idle",
            SessionState::Playing => "playing",
            SessionState::Paused => "paused",
            SessionState::Finished => "finished",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "idle" => Some(SessionState::Idle),
            "playing" => Some(SessionState::Playing),
            "paused" => Some(SessionState::Paused),
            "finished" => Some(SessionState::Finished),
            _ => None,
        }
    }
}

/// 滑动窗口配置
#[derive(Debug, Clone)]
pub struct WindowConfig {
    /// 当前位置之前保留的段数
    pub before: usize,
    /// 当前位置之后预加载的段数
    pub after: usize,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            before: 2,
            after: 3,
        }
    }
}

impl WindowConfig {
    pub fn new(before: usize, after: usize) -> Self {
        Self { before, after }
    }

    /// 计算窗口范围
    pub fn window_range(&self, current_index: usize, total_segments: usize) -> (usize, usize) {
        let start = current_index.saturating_sub(self.before);
        let end = (current_index + self.after).min(total_segments.saturating_sub(1));
        (start, end)
    }
}

/// 播放会话实体（用于持久化）
#[derive(Debug, Clone)]
pub struct SessionRecord {
    pub id: Uuid,
    pub novel_id: Uuid,
    pub voice_id: Uuid,
    pub current_index: usize,
    pub state: SessionState,
    pub window_config: WindowConfig,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_accessed_at: DateTime<Utc>,
}

/// Session Repository Port
#[async_trait]
pub trait SessionRepositoryPort: Send + Sync {
    /// 保存会话
    async fn save(&self, session: &SessionRecord) -> Result<(), RepositoryError>;

    /// 根据 ID 查找会话
    async fn find_by_id(&self, id: Uuid) -> Result<Option<SessionRecord>, RepositoryError>;

    /// 获取所有会话
    async fn find_all(&self) -> Result<Vec<SessionRecord>, RepositoryError>;

    /// 更新会话
    async fn update(&self, session: &SessionRecord) -> Result<(), RepositoryError>;

    /// 删除会话
    async fn delete(&self, id: Uuid) -> Result<(), RepositoryError>;

    /// 获取所有活跃会话
    async fn find_active(&self) -> Result<Vec<SessionRecord>, RepositoryError>;

    /// 获取过期会话（超过指定秒数未访问）
    async fn find_expired(&self, expire_seconds: u64) -> Result<Vec<SessionRecord>, RepositoryError>;
}

// ============================================================================
// Audio Segment Repository
// ============================================================================

/// 音频段落状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioSegmentState {
    /// 等待推理
    Pending,
    /// 推理中
    Inferring,
    /// 已就绪
    Ready,
    /// 推理失败
    Failed,
}

impl AudioSegmentState {
    pub fn as_str(&self) -> &'static str {
        match self {
            AudioSegmentState::Pending => "pending",
            AudioSegmentState::Inferring => "inferring",
            AudioSegmentState::Ready => "ready",
            AudioSegmentState::Failed => "failed",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(AudioSegmentState::Pending),
            "inferring" => Some(AudioSegmentState::Inferring),
            "ready" => Some(AudioSegmentState::Ready),
            "failed" => Some(AudioSegmentState::Failed),
            _ => None,
        }
    }
}

/// 音频段落实体（用于持久化）
#[derive(Debug, Clone)]
pub struct AudioSegmentRecord {
    pub id: Uuid,
    pub session_id: Uuid,
    pub segment_index: usize,
    pub audio_path: Option<PathBuf>,
    pub duration_ms: Option<u32>,
    pub file_size: Option<u64>,
    pub state: AudioSegmentState,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_accessed_at: DateTime<Utc>,
}

/// Audio Segment Repository Port
#[async_trait]
pub trait AudioSegmentRepositoryPort: Send + Sync {
    /// 保存音频段落
    async fn save(&self, segment: &AudioSegmentRecord) -> Result<(), RepositoryError>;

    /// 根据 ID 查找音频段落
    async fn find_by_id(&self, id: Uuid) -> Result<Option<AudioSegmentRecord>, RepositoryError>;

    /// 根据会话和索引查找音频段落
    async fn find_by_session_and_index(
        &self,
        session_id: Uuid,
        index: usize,
    ) -> Result<Option<AudioSegmentRecord>, RepositoryError>;

    /// 更新音频段落
    async fn update(&self, segment: &AudioSegmentRecord) -> Result<(), RepositoryError>;

    /// 删除音频段落
    async fn delete(&self, id: Uuid) -> Result<(), RepositoryError>;

    /// 删除会话的所有音频段落
    async fn delete_by_session(&self, session_id: Uuid) -> Result<usize, RepositoryError>;

    /// 获取会话的所有音频段落
    async fn find_by_session(&self, session_id: Uuid) -> Result<Vec<AudioSegmentRecord>, RepositoryError>;

    /// 获取会话在指定范围内的音频段落
    async fn find_by_session_in_range(
        &self,
        session_id: Uuid,
        start_index: usize,
        end_index: usize,
    ) -> Result<Vec<AudioSegmentRecord>, RepositoryError>;

    /// 获取窗口外的音频段落（用于 GC）
    async fn find_outside_window(
        &self,
        session_id: Uuid,
        window_start: usize,
        window_end: usize,
    ) -> Result<Vec<AudioSegmentRecord>, RepositoryError>;

    /// 更新最后访问时间
    async fn touch(&self, id: Uuid) -> Result<(), RepositoryError>;
}
