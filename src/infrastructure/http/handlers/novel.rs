//! Novel HTTP Handlers - V2 架构

use axum::{
    extract::{Multipart, State},
    Json,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use uuid::Uuid;

use crate::application::{
    CreateNovelFromText, DeleteNovel, GetNovel, GetNovelSegments, ListNovels, ProcessNovelSegments,
};
use crate::infrastructure::http::dto::ApiResponse;
use crate::infrastructure::http::error::ApiError;
use crate::infrastructure::http::state::AppState;

// ============================================================================
// DTOs
// ============================================================================

#[derive(Debug, Serialize)]
pub struct NovelResponse {
    pub id: Uuid,
    pub title: String,
    pub total_segments: usize,
    pub status: String,
    pub created_at: String,
}

/// 异步上传响应 - 立即返回 novel_id，处理完成后通过 WS 通知
#[derive(Debug, Serialize)]
pub struct NovelUploadResponse {
    pub id: Uuid,
    pub title: String,
    pub status: String, // "processing" | "ready" | "failed"
}

#[derive(Debug, Deserialize)]
pub struct GetNovelRequest {
    pub id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct DeleteNovelRequest {
    pub id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct GetNovelSegmentsRequest {
    pub novel_id: Uuid,
    #[serde(default)]
    pub start: usize,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    100
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

/// 删除小说响应
#[derive(Debug, Serialize)]
pub struct DeleteNovelResponse {
    pub id: Uuid,
    pub status: String, // "deleting"
}

// ============================================================================
// Handlers
// ============================================================================

/// 上传小说 TXT 文件（异步处理，立即返回，完成后通过 WS 通知）
pub async fn upload_novel(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<Json<ApiResponse<NovelUploadResponse>>, ApiError> {
    let mut title: Option<String> = None;
    let mut content: Option<String> = None;
    let mut filename: Option<String> = None;

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        ApiError::BadRequest(format!("Failed to read multipart field: {}", e))
    })? {
        let field_name = field.name().unwrap_or_default().to_string();

        match field_name.as_str() {
            "title" => {
                title = Some(
                    field
                        .text()
                        .await
                        .map_err(|e| ApiError::BadRequest(format!("Failed to read title: {}", e)))?,
                );
            }
            "file" => {
                filename = field.file_name().map(|s| s.to_string());

                // 验证文件类型
                let content_type = field.content_type().unwrap_or("application/octet-stream");
                let is_txt = filename
                    .as_ref()
                    .map(|f| f.to_lowercase().ends_with(".txt"))
                    .unwrap_or(false);
                let is_text_type = content_type.contains("text");

                if !is_txt && !is_text_type {
                    return Err(ApiError::BadRequest(
                        "Only TXT files are allowed".to_string(),
                    ));
                }

                let bytes = field
                    .bytes()
                    .await
                    .map_err(|e| ApiError::BadRequest(format!("Failed to read file: {}", e)))?;

                // 验证文件大小（最大 100MB）
                const MAX_SIZE: usize = 100 * 1024 * 1024;
                if bytes.len() > MAX_SIZE {
                    return Err(ApiError::BadRequest(format!(
                        "File too large. Maximum size is {} MB",
                        MAX_SIZE / 1024 / 1024
                    )));
                }

                content = Some(
                    String::from_utf8(bytes.to_vec())
                        .map_err(|_| ApiError::BadRequest("File must be valid UTF-8 text".to_string()))?,
                );
            }
            _ => {}
        }
    }

    let content = content.ok_or_else(|| ApiError::BadRequest("File is required".to_string()))?;

    let title = title.unwrap_or_else(|| {
        filename
            .as_ref()
            .and_then(|f| {
                PathBuf::from(f)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .map(|s| s.to_string())
            })
            .unwrap_or_else(|| "Untitled".to_string())
    });

    // Step 1: 创建 processing 状态的记录，立即返回 ID
    let command = CreateNovelFromText {
        title: title.clone(),
        text: content.clone(),
    };

    let result = state.create_novel_handler.handle(command).await?;
    let novel_id = result.id;
    let novel_title = result.title.clone();

    tracing::info!(
        novel_id = %novel_id,
        title = %novel_title,
        "Novel created (processing)"
    );

    // Step 2: 异步处理分段 + 保存文件 + WS 通知
    let state_clone = state.clone();
    let content_clone = content.clone();
    tokio::spawn(async move {
        let process_command = ProcessNovelSegments {
            novel_id,
            text: content_clone.clone(),
        };

        match state_clone.process_novel_handler.handle(process_command).await {
            Ok(process_result) => {
                // 保存原始文件
                let novels_dir = PathBuf::from("data/novels");
                if let Err(e) = fs::create_dir_all(&novels_dir).await {
                    tracing::warn!("Failed to create novels directory: {}", e);
                } else {
                    let file_path = novels_dir.join(format!("{}.txt", novel_id));
                    if let Err(e) = fs::write(&file_path, &content_clone).await {
                        tracing::warn!("Failed to save novel file: {}", e);
                    }
                }

                tracing::info!(
                    novel_id = %novel_id,
                    title = %process_result.title,
                    segments = process_result.total_segments,
                    "Novel processing completed"
                );

                // 通过 WS 通知客户端
                state_clone.event_publisher.publish_novel_ready(
                    novel_id,
                    &process_result.title,
                    process_result.total_segments,
                );
            }
            Err(e) => {
                tracing::error!(
                    novel_id = %novel_id,
                    error = %e,
                    "Novel processing failed"
                );

                // 通过 WS 通知客户端失败
                state_clone.event_publisher.publish_novel_failed(
                    novel_id,
                    &e.to_string(),
                );
            }
        }
    });

    // 立即返回，状态为 processing
    Ok(Json(ApiResponse::success(NovelUploadResponse {
        id: novel_id,
        title: novel_title,
        status: "processing".to_string(),
    })))
}

/// 获取小说列表
pub async fn list_novels(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<Vec<NovelResponse>>>, ApiError> {
    let result = state.list_novels_handler.handle(ListNovels).await?;

    let responses: Vec<NovelResponse> = result
        .into_iter()
        .map(|n| NovelResponse {
            id: n.id,
            title: n.title,
            total_segments: n.total_segments,
            status: n.status,
            created_at: n.created_at,
        })
        .collect();

    Ok(Json(ApiResponse::success(responses)))
}

/// 获取小说详情
pub async fn get_novel(
    State(state): State<Arc<AppState>>,
    Json(req): Json<GetNovelRequest>,
) -> Result<Json<ApiResponse<NovelResponse>>, ApiError> {
    let query = GetNovel { novel_id: req.id };

    let result = state.get_novel_handler.handle(query).await?;

    Ok(Json(ApiResponse::success(NovelResponse {
        id: result.id,
        title: result.title,
        total_segments: result.total_segments,
        status: result.status,
        created_at: result.created_at,
    })))
}

/// 获取小说段落
pub async fn get_novel_segments(
    State(state): State<Arc<AppState>>,
    Json(req): Json<GetNovelSegmentsRequest>,
) -> Result<Json<ApiResponse<SegmentsResponse>>, ApiError> {
    let query = GetNovelSegments {
        novel_id: req.novel_id,
        start_index: Some(req.start),
        limit: Some(req.limit),
    };

    let result = state.get_novel_segments_handler.handle(query).await?;

    let segments: Vec<SegmentResponse> = result
        .into_iter()
        .map(|s| SegmentResponse {
            index: s.index,
            content: s.content,
            char_count: s.char_count,
        })
        .collect();

    Ok(Json(ApiResponse::success(SegmentsResponse {
        novel_id: req.novel_id,
        total: segments.len(),
        segments,
    })))
}

/// 删除小说（异步处理，立即返回，完成后通过 WS 通知）
pub async fn delete_novel(
    State(state): State<Arc<AppState>>,
    Json(req): Json<DeleteNovelRequest>,
) -> Result<Json<ApiResponse<DeleteNovelResponse>>, ApiError> {
    let novel_id = req.id;

    // 先检查小说是否存在
    let novel = state
        .novel_repo
        .find_by_id(novel_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Novel {} not found", novel_id)))?;

    tracing::info!(novel_id = %novel_id, title = %novel.title, "Novel deleting");

    // 立即发送 deleting 事件
    state.event_publisher.publish_novel_deleting(novel_id);

    // 异步执行删除
    let state_clone = state.clone();
    tokio::spawn(async move {
        let command = DeleteNovel { novel_id };

        match state_clone.delete_novel_handler.handle(command).await {
            Ok(_) => {
                // 删除本地文件
                let file_path = std::path::PathBuf::from("data/novels").join(format!("{}.txt", novel_id));
                if file_path.exists() {
                    if let Err(e) = tokio::fs::remove_file(&file_path).await {
                        tracing::warn!("Failed to delete novel file: {}", e);
                    }
                }

                tracing::info!(novel_id = %novel_id, "Novel deleted");
                state_clone.event_publisher.publish_novel_deleted(novel_id);
            }
            Err(e) => {
                tracing::error!(novel_id = %novel_id, error = %e, "Novel delete failed");
                state_clone.event_publisher.publish_novel_delete_failed(novel_id, &e.to_string());
            }
        }
    });

    // 立即返回
    Ok(Json(ApiResponse::success(DeleteNovelResponse {
        id: novel_id,
        status: "deleting".to_string(),
    })))
}
