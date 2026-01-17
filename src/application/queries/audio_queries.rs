//! Audio Queries - 音频查询
//!
//! 基于 ARCHITECTURE.md V2 设计

use uuid::Uuid;

/// 获取音频查询
#[derive(Debug, Clone)]
pub struct GetAudioQuery {
    pub novel_id: Uuid,
    pub segment_index: u32,
    pub voice_id: Uuid,
}

/// 获取音频响应
#[derive(Debug, Clone)]
pub struct GetAudioResponse {
    pub audio_data: Vec<u8>,
    pub content_type: String,
}
