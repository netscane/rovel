//! Inference Commands - 推理相关命令
//!
//! 基于 ARCHITECTURE.md V2 设计

use crate::application::ports::TaskState;

/// 提交推理任务命令
#[derive(Debug, Clone)]
pub struct SubmitInferCommand {
    pub session_id: String,
    pub segment_indices: Vec<u32>,
}

/// 任务信息
#[derive(Debug, Clone)]
pub struct TaskInfo {
    pub task_id: String,
    pub segment_index: u32,
    pub state: TaskState,
}

/// 提交推理响应
#[derive(Debug, Clone)]
pub struct SubmitInferResponse {
    pub tasks: Vec<TaskInfo>,
}

/// 查询任务状态命令
#[derive(Debug, Clone)]
pub struct QueryTaskStatusCommand {
    pub task_ids: Vec<String>,
}

/// 任务状态信息
#[derive(Debug, Clone)]
pub struct TaskStatusInfo {
    pub task_id: String,
    pub segment_index: u32,
    pub state: TaskState,
    pub error: Option<String>,
}

/// 查询任务状态响应
#[derive(Debug, Clone)]
pub struct QueryTaskStatusResponse {
    pub tasks: Vec<TaskStatusInfo>,
}
