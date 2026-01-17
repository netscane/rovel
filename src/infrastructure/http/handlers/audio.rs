//! Audio Handlers - V2 架构

use axum::{
    body::Body,
    extract::State,
    http::{header, StatusCode},
    response::Response,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::application::GetAudioQuery;
use crate::infrastructure::http::error::ApiError;
use crate::infrastructure::http::state::AppState;

#[derive(Debug, Deserialize)]
pub struct GetAudioRequest {
    pub novel_id: Uuid,
    pub segment_index: u32,
    pub voice_id: Uuid,
}

pub async fn get_audio(
    State(state): State<Arc<AppState>>,
    Json(req): Json<GetAudioRequest>,
) -> Result<Response, ApiError> {
    let query = GetAudioQuery {
        novel_id: req.novel_id,
        segment_index: req.segment_index,
        voice_id: req.voice_id,
    };

    let result = state.get_audio_handler.handle(query).await?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, result.content_type)
        .header(header::CONTENT_LENGTH, result.audio_data.len())
        .body(Body::from(result.audio_data))
        .unwrap())
}
