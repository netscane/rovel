//! Voice Queries - V2 架构

use uuid::Uuid;

/// 获取音色详情查询
#[derive(Debug, Clone)]
pub struct GetVoice {
    pub voice_id: Uuid,
}

/// 列出所有音色查询
#[derive(Debug, Clone)]
pub struct ListVoices;
