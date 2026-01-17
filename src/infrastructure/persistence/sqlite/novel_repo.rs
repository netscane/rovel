//! SQLite Novel Repository

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::FromRow;
use std::path::PathBuf;
use uuid::Uuid;

use super::DbPool;
use crate::application::ports::{
    NovelRecord, NovelRepositoryPort, NovelStatus, RepositoryError, TextSegmentRecord,
};

/// SQLite Novel Repository
pub struct SqliteNovelRepository {
    pool: DbPool,
}

impl SqliteNovelRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
}

#[derive(FromRow)]
struct NovelRow {
    id: String,
    title: String,
    raw_text_path: String,
    total_segments: i64,
    status: String,
    created_at: String,
    updated_at: String,
}

impl TryFrom<NovelRow> for NovelRecord {
    type Error = RepositoryError;

    fn try_from(row: NovelRow) -> Result<Self, Self::Error> {
        Ok(NovelRecord {
            id: Uuid::parse_str(&row.id)
                .map_err(|e| RepositoryError::SerializationError(e.to_string()))?,
            title: row.title,
            raw_text_path: PathBuf::from(row.raw_text_path),
            total_segments: row.total_segments as usize,
            status: NovelStatus::from_str(&row.status).unwrap_or_default(),
            created_at: DateTime::parse_from_rfc3339(&row.created_at)
                .map_err(|e| RepositoryError::SerializationError(e.to_string()))?
                .with_timezone(&Utc),
            updated_at: DateTime::parse_from_rfc3339(&row.updated_at)
                .map_err(|e| RepositoryError::SerializationError(e.to_string()))?
                .with_timezone(&Utc),
        })
    }
}

#[derive(FromRow)]
struct TextSegmentRow {
    id: String,
    novel_id: String,
    segment_index: i64,
    content: String,
    char_count: i64,
}

impl TryFrom<TextSegmentRow> for TextSegmentRecord {
    type Error = RepositoryError;

    fn try_from(row: TextSegmentRow) -> Result<Self, Self::Error> {
        Ok(TextSegmentRecord {
            id: Uuid::parse_str(&row.id)
                .map_err(|e| RepositoryError::SerializationError(e.to_string()))?,
            novel_id: Uuid::parse_str(&row.novel_id)
                .map_err(|e| RepositoryError::SerializationError(e.to_string()))?,
            index: row.segment_index as usize,
            content: row.content,
            char_count: row.char_count as usize,
        })
    }
}

#[async_trait]
impl NovelRepositoryPort for SqliteNovelRepository {
    async fn save(&self, novel: &NovelRecord) -> Result<(), RepositoryError> {
        sqlx::query(
            r#"
            INSERT INTO novels (id, title, raw_text_path, total_segments, status, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                title = excluded.title,
                raw_text_path = excluded.raw_text_path,
                total_segments = excluded.total_segments,
                status = excluded.status,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(novel.id.to_string())
        .bind(&novel.title)
        .bind(novel.raw_text_path.to_string_lossy().to_string())
        .bind(novel.total_segments as i64)
        .bind(novel.status.as_str())
        .bind(novel.created_at.to_rfc3339())
        .bind(novel.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<NovelRecord>, RepositoryError> {
        let row: Option<NovelRow> = sqlx::query_as(
            "SELECT id, title, raw_text_path, total_segments, status, created_at, updated_at FROM novels WHERE id = ?",
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        row.map(NovelRecord::try_from).transpose()
    }

    async fn find_all(&self) -> Result<Vec<NovelRecord>, RepositoryError> {
        let rows: Vec<NovelRow> = sqlx::query_as(
            "SELECT id, title, raw_text_path, total_segments, status, created_at, updated_at FROM novels ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        rows.into_iter().map(NovelRecord::try_from).collect()
    }

    async fn delete(&self, id: Uuid) -> Result<(), RepositoryError> {
        // 使用事务确保原子性
        let mut tx = self.pool.begin().await
            .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        // 删除关联的 audio_segments（通过 sessions）
        sqlx::query(
            "DELETE FROM audio_segments WHERE session_id IN (SELECT id FROM sessions WHERE novel_id = ?)"
        )
        .bind(id.to_string())
        .execute(&mut *tx)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        // 删除关联的 sessions
        sqlx::query("DELETE FROM sessions WHERE novel_id = ?")
            .bind(id.to_string())
            .execute(&mut *tx)
            .await
            .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        // 删除关联的 text_segments
        sqlx::query("DELETE FROM text_segments WHERE novel_id = ?")
            .bind(id.to_string())
            .execute(&mut *tx)
            .await
            .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        // 删除 novel
        sqlx::query("DELETE FROM novels WHERE id = ?")
            .bind(id.to_string())
            .execute(&mut *tx)
            .await
            .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        tx.commit().await
            .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    async fn save_segments(&self, segments: &[TextSegmentRecord]) -> Result<(), RepositoryError> {
        for segment in segments {
            sqlx::query(
                r#"
                INSERT INTO text_segments (id, novel_id, segment_index, content, char_count)
                VALUES (?, ?, ?, ?, ?)
                ON CONFLICT(novel_id, segment_index) DO UPDATE SET
                    content = excluded.content,
                    char_count = excluded.char_count
                "#,
            )
            .bind(segment.id.to_string())
            .bind(segment.novel_id.to_string())
            .bind(segment.index as i64)
            .bind(&segment.content)
            .bind(segment.char_count as i64)
            .execute(&self.pool)
            .await
            .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;
        }

        Ok(())
    }

    async fn find_segments_by_novel_id(
        &self,
        novel_id: Uuid,
    ) -> Result<Vec<TextSegmentRecord>, RepositoryError> {
        let rows: Vec<TextSegmentRow> = sqlx::query_as(
            "SELECT id, novel_id, segment_index, content, char_count FROM text_segments WHERE novel_id = ? ORDER BY segment_index",
        )
        .bind(novel_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        rows.into_iter().map(TextSegmentRecord::try_from).collect()
    }

    async fn find_segment(
        &self,
        novel_id: Uuid,
        index: usize,
    ) -> Result<Option<TextSegmentRecord>, RepositoryError> {
        let row: Option<TextSegmentRow> = sqlx::query_as(
            "SELECT id, novel_id, segment_index, content, char_count FROM text_segments WHERE novel_id = ? AND segment_index = ?",
        )
        .bind(novel_id.to_string())
        .bind(index as i64)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        row.map(TextSegmentRecord::try_from).transpose()
    }

    async fn find_segments_paginated(
        &self,
        novel_id: Uuid,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<TextSegmentRecord>, RepositoryError> {
        let rows: Vec<TextSegmentRow> = sqlx::query_as(
            "SELECT id, novel_id, segment_index, content, char_count FROM text_segments WHERE novel_id = ? ORDER BY segment_index LIMIT ? OFFSET ?",
        )
        .bind(novel_id.to_string())
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        rows.into_iter().map(TextSegmentRecord::try_from).collect()
    }

    async fn find_segments_by_indices(
        &self,
        novel_id: Uuid,
        indices: &[u32],
    ) -> Result<Vec<TextSegmentRecord>, RepositoryError> {
        if indices.is_empty() {
            return Ok(Vec::new());
        }

        // 构建 IN 子句的占位符
        let placeholders: Vec<String> = indices.iter().map(|_| "?".to_string()).collect();
        let query = format!(
            "SELECT id, novel_id, segment_index, content, char_count FROM text_segments WHERE novel_id = ? AND segment_index IN ({}) ORDER BY segment_index",
            placeholders.join(", ")
        );

        let mut sql_query = sqlx::query_as::<_, TextSegmentRow>(&query)
            .bind(novel_id.to_string());
        
        for idx in indices {
            sql_query = sql_query.bind(*idx as i64);
        }

        let rows: Vec<TextSegmentRow> = sql_query
            .fetch_all(&self.pool)
            .await
            .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        rows.into_iter().map(TextSegmentRecord::try_from).collect()
    }

    async fn update_status(
        &self,
        id: Uuid,
        status: NovelStatus,
        total_segments: usize,
    ) -> Result<(), RepositoryError> {
        sqlx::query(
            r#"
            UPDATE novels 
            SET status = ?, total_segments = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(status.as_str())
        .bind(total_segments as i64)
        .bind(Utc::now().to_rfc3339())
        .bind(id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    async fn save_segments_batch(&self, segments: &[TextSegmentRecord]) -> Result<(), RepositoryError> {
        if segments.is_empty() {
            return Ok(());
        }

        // 使用事务批量插入，每批 500 条
        const BATCH_SIZE: usize = 500;
        
        for chunk in segments.chunks(BATCH_SIZE) {
            // 构建批量 INSERT 语句
            let mut query = String::from(
                "INSERT INTO text_segments (id, novel_id, segment_index, content, char_count) VALUES "
            );
            
            let placeholders: Vec<String> = chunk
                .iter()
                .map(|_| "(?, ?, ?, ?, ?)".to_string())
                .collect();
            query.push_str(&placeholders.join(", "));
            
            query.push_str(
                " ON CONFLICT(novel_id, segment_index) DO UPDATE SET content = excluded.content, char_count = excluded.char_count"
            );

            let mut sql_query = sqlx::query(&query);
            
            for segment in chunk {
                sql_query = sql_query
                    .bind(segment.id.to_string())
                    .bind(segment.novel_id.to_string())
                    .bind(segment.index as i64)
                    .bind(&segment.content)
                    .bind(segment.char_count as i64);
            }

            sql_query
                .execute(&self.pool)
                .await
                .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;
        }

        Ok(())
    }
}
