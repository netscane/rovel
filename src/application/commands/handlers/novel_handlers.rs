//! Novel Command Handlers - V2 架构

use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

use crate::application::commands::{CreateNovelFromText, DeleteNovel, ProcessNovelSegments};
use crate::application::error::ApplicationError;
use crate::application::ports::{NovelRecord, NovelRepositoryPort, NovelStatus, TextSegmentRecord};
use crate::domain::segment_text;
use crate::domain::SegmentConfig;

// ============================================================================
// CreateNovelFromText (Step 1: Create processing record)
// ============================================================================

/// 创建小说响应（立即返回，status=processing）
#[derive(Debug, Clone)]
pub struct CreateNovelResponse {
    pub id: Uuid,
    pub title: String,
    pub status: NovelStatus,
}

/// CreateNovelFromText Handler - 创建 processing 状态的记录
pub struct CreateNovelFromTextHandler {
    novel_repo: Arc<dyn NovelRepositoryPort>,
}

impl CreateNovelFromTextHandler {
    pub fn new(novel_repo: Arc<dyn NovelRepositoryPort>) -> Self {
        Self { novel_repo }
    }

    /// 第一步：创建 processing 状态的小说记录，立即返回 ID
    pub async fn handle(&self, command: CreateNovelFromText) -> Result<CreateNovelResponse, ApplicationError> {
        let novel_id = Uuid::new_v4();
        let now = Utc::now();

        // 创建 processing 状态的小说记录
        let novel = NovelRecord {
            id: novel_id,
            title: command.title.clone(),
            raw_text_path: std::path::PathBuf::new(),
            total_segments: 0, // 待处理
            status: NovelStatus::Processing,
            created_at: now,
            updated_at: now,
        };

        self.novel_repo.save(&novel).await?;

        tracing::info!(
            novel_id = %novel_id,
            title = %command.title,
            "Novel created (processing)"
        );

        Ok(CreateNovelResponse {
            id: novel_id,
            title: command.title,
            status: NovelStatus::Processing,
        })
    }
}

// ============================================================================
// ProcessNovelSegments (Step 2: Async segmentation)
// ============================================================================

/// 处理分段响应
#[derive(Debug, Clone)]
pub struct ProcessNovelResponse {
    pub id: Uuid,
    pub title: String,
    pub total_segments: usize,
}

/// ProcessNovelSegments Handler - 异步处理分段
pub struct ProcessNovelSegmentsHandler {
    novel_repo: Arc<dyn NovelRepositoryPort>,
}

impl ProcessNovelSegmentsHandler {
    pub fn new(novel_repo: Arc<dyn NovelRepositoryPort>) -> Self {
        Self { novel_repo }
    }

    /// 第二步：处理文本分段，更新状态为 ready
    pub async fn handle(&self, command: ProcessNovelSegments) -> Result<ProcessNovelResponse, ApplicationError> {
        let novel_id = command.novel_id;

        // 获取小说记录
        let novel = self
            .novel_repo
            .find_by_id(novel_id)
            .await?
            .ok_or_else(|| ApplicationError::not_found("Novel", novel_id))?;

        // 按行+标点分段
        let segments = segment_text(&command.text, &SegmentConfig::default());
        let total_segments = segments.len();

        // 创建分段记录
        let segment_records: Vec<TextSegmentRecord> = segments
            .iter()
            .enumerate()
            .map(|(index, content)| TextSegmentRecord {
                id: Uuid::new_v4(),
                novel_id,
                index,
                content: content.to_string(),
                char_count: content.chars().count(),
            })
            .collect();

        // 批量插入分段
        self.novel_repo.save_segments_batch(&segment_records).await?;

        // 更新小说状态为 ready
        self.novel_repo
            .update_status(novel_id, NovelStatus::Ready, total_segments)
            .await?;

        tracing::info!(
            novel_id = %novel_id,
            title = %novel.title,
            total_segments = total_segments,
            "Novel segments processed"
        );

        Ok(ProcessNovelResponse {
            id: novel_id,
            title: novel.title,
            total_segments,
        })
    }
}

// ============================================================================
// DeleteNovel
// ============================================================================

/// DeleteNovel Handler
pub struct DeleteNovelHandler {
    novel_repo: Arc<dyn NovelRepositoryPort>,
}

impl DeleteNovelHandler {
    pub fn new(novel_repo: Arc<dyn NovelRepositoryPort>) -> Self {
        Self { novel_repo }
    }

    pub async fn handle(&self, command: DeleteNovel) -> Result<(), ApplicationError> {
        let novel_id = command.novel_id;

        // 检查小说是否存在
        let novel = self
            .novel_repo
            .find_by_id(novel_id)
            .await?
            .ok_or_else(|| ApplicationError::not_found("Novel", novel_id))?;

        self.novel_repo.delete(novel_id).await?;

        tracing::info!(
            novel_id = %novel_id,
            title = %novel.title,
            "Novel deleted"
        );

        Ok(())
    }
}
