//! Voice HTTP Handlers - V2 架构

use axum::{
    body::Body,
    extract::{Multipart, Path, State},
    http::{header, StatusCode},
    response::Response,
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tokio_util::io::ReaderStream;
use uuid::Uuid;

use crate::application::{CreateVoice, DeleteVoice, GetVoice, ListVoices};
use crate::infrastructure::http::dto::{ApiResponse, Empty};
use crate::infrastructure::http::error::ApiError;
use crate::infrastructure::http::state::AppState;

// ============================================================================
// DTOs
// ============================================================================

#[derive(Debug, Serialize)]
pub struct VoiceResponse {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct GetVoiceRequest {
    pub id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct DeleteVoiceRequest {
    pub id: Uuid,
}

// ============================================================================
// Handlers
// ============================================================================

/// 上传音色
pub async fn upload_voice(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<Json<ApiResponse<VoiceResponse>>, ApiError> {
    let mut name: Option<String> = None;
    let mut description: Option<String> = None;
    let mut audio_data: Option<Vec<u8>> = None;
    let mut audio_ext: Option<String> = None;

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        ApiError::BadRequest(format!("Failed to read multipart field: {}", e))
    })? {
        let field_name = field.name().unwrap_or_default().to_string();

        match field_name.as_str() {
            "name" => {
                name = Some(
                    field
                        .text()
                        .await
                        .map_err(|e| ApiError::BadRequest(format!("Failed to read name: {}", e)))?,
                );
            }
            "description" => {
                description = Some(
                    field
                        .text()
                        .await
                        .map_err(|e| ApiError::BadRequest(format!("Failed to read description: {}", e)))?,
                );
            }
            "file" => {
                let filename = field.file_name().map(|s| s.to_string());
                audio_ext = filename.as_ref().and_then(|f| {
                    PathBuf::from(f)
                        .extension()
                        .and_then(|e| e.to_str())
                        .map(|s| s.to_lowercase())
                });

                // 验证音频格式
                let valid_exts = ["wav", "mp3", "flac", "ogg"];
                if !audio_ext
                    .as_ref()
                    .map(|e| valid_exts.contains(&e.as_str()))
                    .unwrap_or(false)
                {
                    return Err(ApiError::BadRequest(
                        "Only WAV, MP3, FLAC, OGG audio files are allowed".to_string(),
                    ));
                }

                audio_data = Some(
                    field
                        .bytes()
                        .await
                        .map_err(|e| ApiError::BadRequest(format!("Failed to read file: {}", e)))?
                        .to_vec(),
                );
            }
            _ => {}
        }
    }

    let name = name.ok_or_else(|| ApiError::BadRequest("Name is required".to_string()))?;
    let audio_data =
        audio_data.ok_or_else(|| ApiError::BadRequest("Audio file is required".to_string()))?;
    let audio_ext = audio_ext.unwrap_or_else(|| "wav".to_string());

    // 保存音频文件
    let voice_id = Uuid::new_v4();
    let voices_dir = PathBuf::from("data/voices");
    fs::create_dir_all(&voices_dir)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to create voices directory: {}", e)))?;

    let audio_path = voices_dir.join(format!("{}.{}", voice_id, audio_ext));
    fs::write(&audio_path, &audio_data)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to save audio file: {}", e)))?;

    // 创建音色
    let command = CreateVoice {
        name: name.clone(),
        reference_audio_path: audio_path.clone(),
        description: description.clone(),
    };

    let result = state.create_voice_handler.handle(command).await?;

    tracing::info!(
        voice_id = %result.id,
        name = %result.name,
        "Voice uploaded"
    );

    Ok(Json(ApiResponse::success(VoiceResponse {
        id: result.id,
        name: result.name,
        description: result.description,
        created_at: Utc::now().to_rfc3339(),
    })))
}

/// 获取音色列表
pub async fn list_voices(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<Vec<VoiceResponse>>>, ApiError> {
    let result = state.list_voices_handler.handle(ListVoices).await?;

    let responses: Vec<VoiceResponse> = result
        .into_iter()
        .map(|v| VoiceResponse {
            id: v.id,
            name: v.name,
            description: v.description,
            created_at: v.created_at,
        })
        .collect();

    Ok(Json(ApiResponse::success(responses)))
}

/// 获取音色详情
pub async fn get_voice(
    State(state): State<Arc<AppState>>,
    Json(req): Json<GetVoiceRequest>,
) -> Result<Json<ApiResponse<VoiceResponse>>, ApiError> {
    let query = GetVoice { voice_id: req.id };

    let result = state.get_voice_handler.handle(query).await?;

    Ok(Json(ApiResponse::success(VoiceResponse {
        id: result.id,
        name: result.name,
        description: result.description,
        created_at: result.created_at,
    })))
}

/// 删除音色（同步，完成后广播 WS 事件）
pub async fn delete_voice(
    State(state): State<Arc<AppState>>,
    Json(req): Json<DeleteVoiceRequest>,
) -> Result<Json<ApiResponse<Empty>>, ApiError> {
    let voice_id = req.id;

    // 获取音色信息
    let voice = state
        .voice_repo
        .find_by_id(voice_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Voice {} not found", voice_id)))?;

    let audio_path = voice.reference_audio_path.clone();

    // 删除数据库记录
    let command = DeleteVoice { voice_id };
    state.delete_voice_handler.handle(command).await?;

    // 删除音频文件
    if audio_path.exists() {
        if let Err(e) = tokio::fs::remove_file(&audio_path).await {
            tracing::warn!("Failed to delete voice audio file: {}", e);
        }
    }

    tracing::info!(voice_id = %voice_id, "Voice deleted");

    // 广播事件通知其他客户端
    state.event_publisher.publish_voice_deleted(voice_id);

    Ok(Json(ApiResponse::ok()))
}

/// 下载音色参考音频（供外部 TTS 服务使用）
pub async fn download_voice_audio(
    State(state): State<Arc<AppState>>,
    Path(voice_id): Path<Uuid>,
) -> Result<Response, ApiError> {
    // 直接从 repository 查询以获取 reference_audio_path
    let voice = state
        .voice_repo
        .find_by_id(voice_id)
        .await
        .map_err(|e| ApiError::Internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::NotFound(format!("Voice not found: {}", voice_id)))?;

    // 获取音频文件路径
    let audio_path = &voice.reference_audio_path;
    if !audio_path.exists() {
        return Err(ApiError::NotFound(format!(
            "Voice audio file not found: {}",
            voice_id
        )));
    }

    // 打开文件
    let file = tokio::fs::File::open(&audio_path)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to open audio file: {}", e)))?;

    // 获取文件大小
    let metadata = file
        .metadata()
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to get file metadata: {}", e)))?;
    let file_size = metadata.len();

    // 检测 Content-Type
    let content_type = match audio_path.extension().and_then(|e| e.to_str()) {
        Some("wav") => "audio/wav",
        Some("mp3") => "audio/mpeg",
        Some("flac") => "audio/flac",
        Some("ogg") => "audio/ogg",
        _ => "application/octet-stream",
    };

    // 流式返回文件内容
    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CONTENT_LENGTH, file_size)
        .header(
            header::CONTENT_DISPOSITION,
            format!(
                "attachment; filename=\"{}.{}\"",
                voice_id,
                audio_path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("wav")
            ),
        )
        .body(body)
        .unwrap())
}
