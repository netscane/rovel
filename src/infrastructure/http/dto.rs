//! Data Transfer Objects - V2 架构

use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ============================================================================
// 统一响应结构
// ============================================================================

/// 统一 API 响应格式
#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub errno: i32,
    pub error: String,
    pub data: Option<T>,
}

impl<T: Serialize> ApiResponse<T> {
    /// 成功响应
    pub fn success(data: T) -> Self {
        Self {
            errno: 0,
            error: String::new(),
            data: Some(data),
        }
    }

    /// 错误响应
    #[allow(dead_code)]
    pub fn error(errno: i32, error: impl Into<String>) -> ApiResponse<()> {
        ApiResponse {
            errno,
            error: error.into(),
            data: None,
        }
    }
}

/// 空数据响应
#[derive(Debug, Serialize)]
pub struct Empty {}

impl ApiResponse<Empty> {
    /// 成功但无数据
    pub fn ok() -> Self {
        Self {
            errno: 0,
            error: String::new(),
            data: Some(Empty {}),
        }
    }
}

// ============================================================================
// Novel DTOs
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct CreateNovelRequest {
    pub title: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct DeleteNovelRequest {
    pub id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct GetNovelRequest {
    pub id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct NovelResponse {
    pub id: Uuid,
    pub title: String,
    pub total_segments: usize,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct SegmentResponse {
    pub index: usize,
    pub content: String,
    pub char_count: usize,
}

#[derive(Debug, Serialize)]
pub struct SegmentsResponse {
    pub novel_id: Uuid,
    pub total: usize,
    pub segments: Vec<SegmentResponse>,
}

// ============================================================================
// Voice DTOs
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct GetVoiceRequest {
    pub id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct DeleteVoiceRequest {
    pub id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct VoiceResponse {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub created_at: String,
}
