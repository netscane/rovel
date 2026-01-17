//! Task Manager Port - 推理任务管理
//!
//! 定义任务管理的抽象接口，具体实现在 infrastructure/memory 层

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

/// Task Manager 错误
#[derive(Debug, Error)]
pub enum TaskError {
    #[error("Task not found: {0}")]
    NotFound(String),

    #[error("Task already exists: {0}")]
    AlreadyExists(String),

    #[error("Invalid state transition: {0}")]
    InvalidStateTransition(String),
}

/// 任务状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskState {
    /// 等待推理
    Pending,
    /// 正在推理
    Inferring,
    /// 推理完成
    Ready,
    /// 推理失败
    Failed,
    /// 已取消
    Cancelled,
}

impl TaskState {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskState::Pending => "pending",
            TaskState::Inferring => "inferring",
            TaskState::Ready => "ready",
            TaskState::Failed => "failed",
            TaskState::Cancelled => "cancelled",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(TaskState::Pending),
            "inferring" => Some(TaskState::Inferring),
            "ready" => Some(TaskState::Ready),
            "failed" => Some(TaskState::Failed),
            "cancelled" => Some(TaskState::Cancelled),
            _ => None,
        }
    }
}

/// 推理任务
#[derive(Debug, Clone)]
pub struct InferenceTask {
    pub task_id: String,
    pub session_id: String,
    pub novel_id: Uuid,
    pub voice_id: Uuid,
    pub segment_index: u32,
    pub segment_content: String,
    pub state: TaskState,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
}

impl InferenceTask {
    pub fn new(
        session_id: String,
        novel_id: Uuid,
        voice_id: Uuid,
        segment_index: u32,
        segment_content: String,
    ) -> Self {
        Self {
            task_id: Uuid::new_v4().to_string(),
            session_id,
            novel_id,
            voice_id,
            segment_index,
            segment_content,
            state: TaskState::Pending,
            created_at: Utc::now(),
            completed_at: None,
            error_message: None,
        }
    }
}

/// Task Manager Port
///
/// 管理推理任务的生命周期，所有状态存储在内存中
pub trait TaskManagerPort: Send + Sync {
    /// 提交任务到队列
    fn submit(&self, tasks: Vec<InferenceTask>) -> Result<Vec<String>, TaskError>;

    /// 取消会话的所有 pending 任务，返回取消数量
    fn cancel_pending(&self, session_id: &str) -> usize;

    /// 检查任务是否已取消
    fn is_cancelled(&self, task_id: &str) -> bool;

    /// 获取任务状态
    fn get_state(&self, task_id: &str) -> Option<TaskState>;

    /// 设置任务状态
    fn set_state(&self, task_id: &str, state: TaskState) -> Result<(), TaskError>;

    /// 设置任务失败并记录错误
    fn set_failed(&self, task_id: &str, error: String) -> Result<(), TaskError>;

    /// 获取任务
    fn get_task(&self, task_id: &str) -> Option<InferenceTask>;

    /// 获取会话的所有任务
    fn get_tasks_by_session(&self, session_id: &str) -> Vec<InferenceTask>;

    /// 清理会话的所有任务
    fn cleanup_session(&self, session_id: &str);
}
