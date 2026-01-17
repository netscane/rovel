//! Session Commands - 会话相关命令
//!
//! 基于 ARCHITECTURE.md V2 设计

use uuid::Uuid;

/// 开始播放命令 - 创建或复用会话
#[derive(Debug, Clone)]
pub struct PlayCommand {
    pub novel_id: Uuid,
    pub voice_id: Uuid,
    pub start_index: u32,
}

/// 开始播放响应
#[derive(Debug, Clone)]
pub struct PlayResponse {
    pub session_id: String,
    pub novel_id: Uuid,
    pub voice_id: Uuid,
    pub current_index: u32,
}

/// Seek 命令 - 跳转位置并取消 pending 任务
#[derive(Debug, Clone)]
pub struct SeekCommand {
    pub session_id: String,
    pub segment_index: u32,
}

/// Seek 响应
#[derive(Debug, Clone)]
pub struct SeekResponse {
    pub session_id: String,
    pub current_index: u32,
    pub cancelled_count: usize,
}

/// 切换音色命令 - 取消所有任务
#[derive(Debug, Clone)]
pub struct ChangeVoiceCommand {
    pub session_id: String,
    pub voice_id: Uuid,
}

/// 切换音色响应
#[derive(Debug, Clone)]
pub struct ChangeVoiceResponse {
    pub session_id: String,
    pub voice_id: Uuid,
    pub cancelled_count: usize,
}

/// 关闭会话命令
#[derive(Debug, Clone)]
pub struct CloseSessionCommand {
    pub session_id: String,
}

/// 关闭会话响应
#[derive(Debug, Clone)]
pub struct CloseSessionResponse {
    pub session_id: String,
}
