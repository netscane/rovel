//! Inference Handlers - V2 架构

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::application::{QueryTaskStatusCommand, SubmitInferCommand};
use crate::infrastructure::http::dto::ApiResponse;
use crate::infrastructure::http::error::ApiError;
use crate::infrastructure::http::state::AppState;

// ============================================================================
// Submit Inference
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct SubmitInferRequest {
    pub session_id: String,
    pub segment_indices: Vec<u32>,
}

#[derive(Debug, Serialize)]
pub struct TaskInfoDto {
    pub task_id: String,
    pub segment_index: u32,
    pub state: String,
}

#[derive(Debug, Serialize)]
pub struct SubmitInferResponseDto {
    pub tasks: Vec<TaskInfoDto>,
}

pub async fn submit_infer(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SubmitInferRequest>,
) -> Result<Json<ApiResponse<SubmitInferResponseDto>>, ApiError> {
    let cmd = SubmitInferCommand {
        session_id: req.session_id,
        segment_indices: req.segment_indices,
    };

    let result = state.submit_infer_handler.handle(cmd).await?;

    Ok(Json(ApiResponse::success(SubmitInferResponseDto {
        tasks: result
            .tasks
            .into_iter()
            .map(|t| TaskInfoDto {
                task_id: t.task_id,
                segment_index: t.segment_index,
                state: t.state.as_str().to_string(),
            })
            .collect(),
    })))
}

// ============================================================================
// Query Task Status
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct QueryTaskStatusRequest {
    pub task_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct TaskStatusInfoDto {
    pub task_id: String,
    pub segment_index: u32,
    pub state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct QueryTaskStatusResponseDto {
    pub tasks: Vec<TaskStatusInfoDto>,
}

pub async fn query_task_status(
    State(state): State<Arc<AppState>>,
    Json(req): Json<QueryTaskStatusRequest>,
) -> Result<Json<ApiResponse<QueryTaskStatusResponseDto>>, ApiError> {
    let cmd = QueryTaskStatusCommand {
        task_ids: req.task_ids,
    };

    let result = state.query_task_status_handler.handle(cmd);

    Ok(Json(ApiResponse::success(QueryTaskStatusResponseDto {
        tasks: result
            .tasks
            .into_iter()
            .map(|t| TaskStatusInfoDto {
                task_id: t.task_id,
                segment_index: t.segment_index,
                state: t.state.as_str().to_string(),
                error: t.error,
            })
            .collect(),
    })))
}
