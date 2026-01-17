//! SQLite Audio Segment Repository

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::FromRow;
use std::path::PathBuf;
use uuid::Uuid;

use super::DbPool;
use crate::application::ports::{
    AudioSegmentRecord, AudioSegmentRepositoryPort, AudioSegmentState, RepositoryError,
};

/// SQLite Audio Segment Repository
pub struct SqliteAudioSegmentRepository {
    pool: DbPool,
}

impl SqliteAudioSegmentRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
}

#[derive(FromRow)]
struct AudioSegmentRow {
    id: String,
    session_id: String,
    segment_index: i64,
    audio_path: Option<String>,
    duration_ms: Option<i64>,
    file_size: Option<i64>,
    state: String,
    error_message: Option<String>,
    created_at: String,
    last_accessed_at: String,
}

impl TryFrom<AudioSegmentRow> for AudioSegmentRecord {
    type Error = RepositoryError;

    fn try_from(row: AudioSegmentRow) -> Result<Self, Self::Error> {
        Ok(AudioSegmentRecord {
            id: Uuid::parse_str(&row.id)
                .map_err(|e| RepositoryError::SerializationError(e.to_string()))?,
            session_id: Uuid::parse_str(&row.session_id)
                .map_err(|e| RepositoryError::SerializationError(e.to_string()))?,
            segment_index: row.segment_index as usize,
            audio_path: row.audio_path.map(PathBuf::from),
            duration_ms: row.duration_ms.map(|d| d as u32),
            file_size: row.file_size.map(|s| s as u64),
            state: AudioSegmentState::from_str(&row.state).unwrap_or(AudioSegmentState::Pending),
            error_message: row.error_message,
            created_at: DateTime::parse_from_rfc3339(&row.created_at)
                .map_err(|e| RepositoryError::SerializationError(e.to_string()))?
                .with_timezone(&Utc),
            last_accessed_at: DateTime::parse_from_rfc3339(&row.last_accessed_at)
                .map_err(|e| RepositoryError::SerializationError(e.to_string()))?
                .with_timezone(&Utc),
        })
    }
}

#[async_trait]
impl AudioSegmentRepositoryPort for SqliteAudioSegmentRepository {
    async fn save(&self, segment: &AudioSegmentRecord) -> Result<(), RepositoryError> {
        sqlx::query(
            r#"
            INSERT INTO audio_segments (id, session_id, segment_index, audio_path, duration_ms, file_size, state, error_message, created_at, last_accessed_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(session_id, segment_index) DO UPDATE SET
                audio_path = excluded.audio_path,
                duration_ms = excluded.duration_ms,
                file_size = excluded.file_size,
                state = excluded.state,
                error_message = excluded.error_message,
                last_accessed_at = excluded.last_accessed_at
            "#,
        )
        .bind(segment.id.to_string())
        .bind(segment.session_id.to_string())
        .bind(segment.segment_index as i64)
        .bind(segment.audio_path.as_ref().map(|p| p.to_string_lossy().to_string()))
        .bind(segment.duration_ms.map(|d| d as i64))
        .bind(segment.file_size.map(|s| s as i64))
        .bind(segment.state.as_str())
        .bind(&segment.error_message)
        .bind(segment.created_at.to_rfc3339())
        .bind(segment.last_accessed_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<AudioSegmentRecord>, RepositoryError> {
        let row: Option<AudioSegmentRow> = sqlx::query_as(
            "SELECT id, session_id, segment_index, audio_path, duration_ms, file_size, state, error_message, created_at, last_accessed_at FROM audio_segments WHERE id = ?",
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        row.map(AudioSegmentRecord::try_from).transpose()
    }

    async fn find_by_session_and_index(
        &self,
        session_id: Uuid,
        index: usize,
    ) -> Result<Option<AudioSegmentRecord>, RepositoryError> {
        let row: Option<AudioSegmentRow> = sqlx::query_as(
            "SELECT id, session_id, segment_index, audio_path, duration_ms, file_size, state, error_message, created_at, last_accessed_at FROM audio_segments WHERE session_id = ? AND segment_index = ?",
        )
        .bind(session_id.to_string())
        .bind(index as i64)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        row.map(AudioSegmentRecord::try_from).transpose()
    }

    async fn update(&self, segment: &AudioSegmentRecord) -> Result<(), RepositoryError> {
        sqlx::query(
            r#"
            UPDATE audio_segments SET
                audio_path = ?,
                duration_ms = ?,
                file_size = ?,
                state = ?,
                error_message = ?,
                last_accessed_at = ?
            WHERE id = ?
            "#,
        )
        .bind(segment.audio_path.as_ref().map(|p| p.to_string_lossy().to_string()))
        .bind(segment.duration_ms.map(|d| d as i64))
        .bind(segment.file_size.map(|s| s as i64))
        .bind(segment.state.as_str())
        .bind(&segment.error_message)
        .bind(segment.last_accessed_at.to_rfc3339())
        .bind(segment.id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<(), RepositoryError> {
        sqlx::query("DELETE FROM audio_segments WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    async fn delete_by_session(&self, session_id: Uuid) -> Result<usize, RepositoryError> {
        let result = sqlx::query("DELETE FROM audio_segments WHERE session_id = ?")
            .bind(session_id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        Ok(result.rows_affected() as usize)
    }

    async fn find_by_session(
        &self,
        session_id: Uuid,
    ) -> Result<Vec<AudioSegmentRecord>, RepositoryError> {
        let rows: Vec<AudioSegmentRow> = sqlx::query_as(
            "SELECT id, session_id, segment_index, audio_path, duration_ms, file_size, state, error_message, created_at, last_accessed_at FROM audio_segments WHERE session_id = ? ORDER BY segment_index",
        )
        .bind(session_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        rows.into_iter().map(AudioSegmentRecord::try_from).collect()
    }

    async fn find_by_session_in_range(
        &self,
        session_id: Uuid,
        start_index: usize,
        end_index: usize,
    ) -> Result<Vec<AudioSegmentRecord>, RepositoryError> {
        let rows: Vec<AudioSegmentRow> = sqlx::query_as(
            "SELECT id, session_id, segment_index, audio_path, duration_ms, file_size, state, error_message, created_at, last_accessed_at FROM audio_segments WHERE session_id = ? AND segment_index >= ? AND segment_index <= ? ORDER BY segment_index",
        )
        .bind(session_id.to_string())
        .bind(start_index as i64)
        .bind(end_index as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        rows.into_iter().map(AudioSegmentRecord::try_from).collect()
    }

    async fn find_outside_window(
        &self,
        session_id: Uuid,
        window_start: usize,
        window_end: usize,
    ) -> Result<Vec<AudioSegmentRecord>, RepositoryError> {
        let rows: Vec<AudioSegmentRow> = sqlx::query_as(
            "SELECT id, session_id, segment_index, audio_path, duration_ms, file_size, state, error_message, created_at, last_accessed_at FROM audio_segments WHERE session_id = ? AND (segment_index < ? OR segment_index > ?) ORDER BY segment_index",
        )
        .bind(session_id.to_string())
        .bind(window_start as i64)
        .bind(window_end as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        rows.into_iter().map(AudioSegmentRecord::try_from).collect()
    }

    async fn touch(&self, id: Uuid) -> Result<(), RepositoryError> {
        sqlx::query("UPDATE audio_segments SET last_accessed_at = ? WHERE id = ?")
            .bind(Utc::now().to_rfc3339())
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        Ok(())
    }
}
