//! Voice Commands - V2 架构

use std::path::PathBuf;
use uuid::Uuid;

/// 创建音色命令
#[derive(Debug, Clone)]
pub struct CreateVoice {
    pub name: String,
    pub reference_audio_path: PathBuf,
    pub description: Option<String>,
}

/// 删除音色命令
#[derive(Debug, Clone)]
pub struct DeleteVoice {
    pub voice_id: Uuid,
}
