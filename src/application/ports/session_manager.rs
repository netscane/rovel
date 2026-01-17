//! Session Manager Port - 会话生命周期管理
//!
//! 定义会话管理的抽象接口，具体实现在 infrastructure/memory 层

use chrono::{DateTime, Utc};
use thiserror::Error;
use uuid::Uuid;

/// Session Manager 错误
#[derive(Debug, Error)]
pub enum SessionError {
    #[error("Session not found: {0}")]
    NotFound(String),

    #[error("Session already exists: {0}")]
    AlreadyExists(String),

    #[error("Session expired: {0}")]
    Expired(String),

    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
}

/// 会话状态（in-memory）
#[derive(Debug, Clone)]
pub struct Session {
    pub id: String,
    pub novel_id: Uuid,
    pub voice_id: Uuid,
    pub current_index: u32,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
}

impl Session {
    pub fn new(novel_id: Uuid, voice_id: Uuid, start_index: u32) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            novel_id,
            voice_id,
            current_index: start_index,
            created_at: now,
            last_activity: now,
        }
    }
}

/// Session Manager Port
///
/// 管理播放会话的生命周期，所有状态存储在内存中
pub trait SessionManagerPort: Send + Sync {
    /// 创建新会话
    fn create(&self, session: Session) -> Result<String, SessionError>;

    /// 获取会话
    fn get(&self, id: &str) -> Result<Session, SessionError>;

    /// 更新当前播放索引
    fn update_index(&self, id: &str, index: u32) -> Result<(), SessionError>;

    /// 更新音色
    fn update_voice(&self, id: &str, voice_id: Uuid) -> Result<(), SessionError>;

    /// 检查会话是否有效
    fn is_valid(&self, id: &str) -> bool;

    /// 关闭会话
    fn close(&self, id: &str) -> Result<(), SessionError>;

    /// 更新最后活动时间
    fn touch(&self, id: &str);

    /// 获取所有过期会话的 ID
    fn get_expired_sessions(&self, idle_timeout_secs: u64) -> Vec<String>;

    /// 获取所有会话 ID
    fn list_all(&self) -> Vec<String>;
}
