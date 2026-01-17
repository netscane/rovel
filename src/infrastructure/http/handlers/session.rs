//! Session Handlers - V2 架构

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::application::{
    ChangeVoiceCommand, CloseSessionCommand, PlayCommand, SeekCommand,
};
use crate::infrastructure::http::dto::ApiResponse;
use crate::infrastructure::http::error::ApiError;
use crate::infrastructure::http::state::AppState;

// ============================================================================
// Play
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct PlayRequest {
    pub novel_id: Uuid,
    pub voice_id: Uuid,
    #[serde(default)]
    pub start_index: u32,
}

#[derive(Debug, Serialize)]
pub struct PlayResponseDto {
    pub session_id: String,
    pub novel_id: Uuid,
    pub voice_id: Uuid,
    pub current_index: u32,
}

pub async fn play(
    State(state): State<Arc<AppState>>,
    Json(req): Json<PlayRequest>,
) -> Result<Json<ApiResponse<PlayResponseDto>>, ApiError> {
    let cmd = PlayCommand {
        novel_id: req.novel_id,
        voice_id: req.voice_id,
        start_index: req.start_index,
    };

    let result = state.play_handler.handle(cmd).await?;

    Ok(Json(ApiResponse::success(PlayResponseDto {
        session_id: result.session_id,
        novel_id: result.novel_id,
        voice_id: result.voice_id,
        current_index: result.current_index,
    })))
}

// ============================================================================
// Seek
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct SeekRequest {
    pub session_id: String,
    pub segment_index: u32,
}

#[derive(Debug, Serialize)]
pub struct SeekResponseDto {
    pub session_id: String,
    pub current_index: u32,
    pub cancelled_tasks: usize,
}

pub async fn seek(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SeekRequest>,
) -> Result<Json<ApiResponse<SeekResponseDto>>, ApiError> {
    let cmd = SeekCommand {
        session_id: req.session_id,
        segment_index: req.segment_index,
    };

    let result = state.seek_handler.handle(cmd).await?;

    Ok(Json(ApiResponse::success(SeekResponseDto {
        session_id: result.session_id,
        current_index: result.current_index,
        cancelled_tasks: result.cancelled_count,
    })))
}

// ============================================================================
// Change Voice
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct ChangeVoiceRequest {
    pub session_id: String,
    pub voice_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct ChangeVoiceResponseDto {
    pub session_id: String,
    pub voice_id: Uuid,
    pub cancelled_tasks: usize,
}

pub async fn change_voice(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ChangeVoiceRequest>,
) -> Result<Json<ApiResponse<ChangeVoiceResponseDto>>, ApiError> {
    let cmd = ChangeVoiceCommand {
        session_id: req.session_id,
        voice_id: req.voice_id,
    };

    let result = state.change_voice_handler.handle(cmd).await?;

    Ok(Json(ApiResponse::success(ChangeVoiceResponseDto {
        session_id: result.session_id,
        voice_id: result.voice_id,
        cancelled_tasks: result.cancelled_count,
    })))
}

// ============================================================================
// Close Session
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct CloseSessionRequest {
    pub session_id: String,
}

#[derive(Debug, Serialize)]
pub struct CloseSessionResponseDto {
    pub session_id: String,
}

pub async fn close_session(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CloseSessionRequest>,
) -> Result<Json<ApiResponse<CloseSessionResponseDto>>, ApiError> {
    let cmd = CloseSessionCommand {
        session_id: req.session_id,
    };

    let result = state.close_session_handler.handle(cmd).await?;

    Ok(Json(ApiResponse::success(CloseSessionResponseDto {
        session_id: result.session_id,
    })))
}
