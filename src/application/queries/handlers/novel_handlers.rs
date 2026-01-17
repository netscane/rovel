//! Novel Query Handlers - V2 架构

use std::sync::Arc;
use uuid::Uuid;

use crate::application::error::ApplicationError;
use crate::application::ports::{NovelRecord, NovelRepositoryPort, TextSegmentRecord};
use crate::application::queries::{GetNovel, GetNovelSegments, ListNovels};

// ============================================================================
// Response DTOs
// ============================================================================

/// 小说详情响应
#[derive(Debug, Clone)]
pub struct NovelResponse {
    pub id: Uuid,
    pub title: String,
    pub total_segments: usize,
    pub status: String,
    pub created_at: String,
}

impl From<NovelRecord> for NovelResponse {
    fn from(record: NovelRecord) -> Self {
        Self {
            id: record.id,
            title: record.title,
            total_segments: record.total_segments,
            status: record.status.as_str().to_string(),
            created_at: record.created_at.to_rfc3339(),
        }
    }
}

/// 文本段落响应
#[derive(Debug, Clone)]
pub struct TextSegmentResponse {
    pub index: usize,
    pub content: String,
    pub char_count: usize,
}

impl From<TextSegmentRecord> for TextSegmentResponse {
    fn from(record: TextSegmentRecord) -> Self {
        Self {
            index: record.index,
            content: record.content,
            char_count: record.char_count,
        }
    }
}

// ============================================================================
// Handlers
// ============================================================================

/// GetNovel Handler
pub struct GetNovelHandler {
    novel_repo: Arc<dyn NovelRepositoryPort>,
}

impl GetNovelHandler {
    pub fn new(novel_repo: Arc<dyn NovelRepositoryPort>) -> Self {
        Self { novel_repo }
    }

    pub async fn handle(&self, query: GetNovel) -> Result<NovelResponse, ApplicationError> {
        let novel = self
            .novel_repo
            .find_by_id(query.novel_id)
            .await?
            .ok_or_else(|| ApplicationError::not_found("Novel", query.novel_id))?;

        Ok(NovelResponse::from(novel))
    }
}

/// ListNovels Handler
pub struct ListNovelsHandler {
    novel_repo: Arc<dyn NovelRepositoryPort>,
}

impl ListNovelsHandler {
    pub fn new(novel_repo: Arc<dyn NovelRepositoryPort>) -> Self {
        Self { novel_repo }
    }

    pub async fn handle(&self, _query: ListNovels) -> Result<Vec<NovelResponse>, ApplicationError> {
        let novels = self.novel_repo.find_all().await?;
        Ok(novels.into_iter().map(NovelResponse::from).collect())
    }
}

/// GetNovelSegments Handler
pub struct GetNovelSegmentsHandler {
    novel_repo: Arc<dyn NovelRepositoryPort>,
}

impl GetNovelSegmentsHandler {
    pub fn new(novel_repo: Arc<dyn NovelRepositoryPort>) -> Self {
        Self { novel_repo }
    }

    pub async fn handle(
        &self,
        query: GetNovelSegments,
    ) -> Result<Vec<TextSegmentResponse>, ApplicationError> {
        // 验证小说存在
        self.novel_repo
            .find_by_id(query.novel_id)
            .await?
            .ok_or_else(|| ApplicationError::not_found("Novel", query.novel_id))?;

        // 分页查询
        let offset = query.start_index.unwrap_or(0);
        let limit = query.limit.unwrap_or(100);

        let segments = self
            .novel_repo
            .find_segments_paginated(query.novel_id, offset, limit)
            .await?;

        Ok(segments.into_iter().map(TextSegmentResponse::from).collect())
    }
}
