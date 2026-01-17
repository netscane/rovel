//! SQLite Session Repository

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use sqlx::FromRow;
use uuid::Uuid;

use super::DbPool;
use crate::application::ports::{
    RepositoryError, SessionRecord, SessionRepositoryPort, SessionState, WindowConfig,
};

/// SQLite Session Repository
pub struct SqliteSessionRepository {
    pool: DbPool,
}

impl SqliteSessionRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
}

#[derive(FromRow)]
struct SessionRow {
    id: String,
    novel_id: String,
    voice_id: String,
    current_index: i64,
    state: String,
    window_before: i64,
    window_after: i64,
    created_at: String,
    updated_at: String,
    last_accessed_at: String,
}

impl TryFrom<SessionRow> for SessionRecord {
    type Error = RepositoryError;

    fn try_from(row: SessionRow) -> Result<Self, Self::Error> {
        Ok(SessionRecord {
            id: Uuid::parse_str(&row.id)
                .map_err(|e| RepositoryError::SerializationError(e.to_string()))?,
            novel_id: Uuid::parse_str(&row.novel_id)
                .map_err(|e| RepositoryError::SerializationError(e.to_string()))?,
            voice_id: Uuid::parse_str(&row.voice_id)
                .map_err(|e| RepositoryError::SerializationError(e.to_string()))?,
            current_index: row.current_index as usize,
            state: SessionState::from_str(&row.state).unwrap_or(SessionState::Idle),
            window_config: WindowConfig::new(row.window_before as usize, row.window_after as usize),
            created_at: DateTime::parse_from_rfc3339(&row.created_at)
                .map_err(|e| RepositoryError::SerializationError(e.to_string()))?
                .with_timezone(&Utc),
            updated_at: DateTime::parse_from_rfc3339(&row.updated_at)
                .map_err(|e| RepositoryError::SerializationError(e.to_string()))?
                .with_timezone(&Utc),
            last_accessed_at: DateTime::parse_from_rfc3339(&row.last_accessed_at)
                .map_err(|e| RepositoryError::SerializationError(e.to_string()))?
                .with_timezone(&Utc),
        })
    }
}

#[async_trait]
impl SessionRepositoryPort for SqliteSessionRepository {
    async fn save(&self, session: &SessionRecord) -> Result<(), RepositoryError> {
        sqlx::query(
            r#"
            INSERT INTO sessions (id, novel_id, voice_id, current_index, state, window_before, window_after, created_at, updated_at, last_accessed_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(session.id.to_string())
        .bind(session.novel_id.to_string())
        .bind(session.voice_id.to_string())
        .bind(session.current_index as i64)
        .bind(session.state.as_str())
        .bind(session.window_config.before as i64)
        .bind(session.window_config.after as i64)
        .bind(session.created_at.to_rfc3339())
        .bind(session.updated_at.to_rfc3339())
        .bind(session.last_accessed_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<SessionRecord>, RepositoryError> {
        let row: Option<SessionRow> = sqlx::query_as(
            "SELECT id, novel_id, voice_id, current_index, state, window_before, window_after, created_at, updated_at, last_accessed_at FROM sessions WHERE id = ?",
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        row.map(SessionRecord::try_from).transpose()
    }

    async fn find_all(&self) -> Result<Vec<SessionRecord>, RepositoryError> {
        let rows: Vec<SessionRow> = sqlx::query_as(
            "SELECT id, novel_id, voice_id, current_index, state, window_before, window_after, created_at, updated_at, last_accessed_at FROM sessions ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        rows.into_iter().map(SessionRecord::try_from).collect()
    }

    async fn update(&self, session: &SessionRecord) -> Result<(), RepositoryError> {
        sqlx::query(
            r#"
            UPDATE sessions SET
                current_index = ?,
                state = ?,
                window_before = ?,
                window_after = ?,
                updated_at = ?,
                last_accessed_at = ?
            WHERE id = ?
            "#,
        )
        .bind(session.current_index as i64)
        .bind(session.state.as_str())
        .bind(session.window_config.before as i64)
        .bind(session.window_config.after as i64)
        .bind(session.updated_at.to_rfc3339())
        .bind(session.last_accessed_at.to_rfc3339())
        .bind(session.id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<(), RepositoryError> {
        sqlx::query("DELETE FROM sessions WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    async fn find_active(&self) -> Result<Vec<SessionRecord>, RepositoryError> {
        let rows: Vec<SessionRow> = sqlx::query_as(
            "SELECT id, novel_id, voice_id, current_index, state, window_before, window_after, created_at, updated_at, last_accessed_at FROM sessions WHERE state != 'finished' ORDER BY last_accessed_at DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        rows.into_iter().map(SessionRecord::try_from).collect()
    }

    async fn find_expired(&self, expire_seconds: u64) -> Result<Vec<SessionRecord>, RepositoryError> {
        let expire_time = Utc::now() - Duration::seconds(expire_seconds as i64);

        let rows: Vec<SessionRow> = sqlx::query_as(
            "SELECT id, novel_id, voice_id, current_index, state, window_before, window_after, created_at, updated_at, last_accessed_at FROM sessions WHERE last_accessed_at < ?",
        )
        .bind(expire_time.to_rfc3339())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        rows.into_iter().map(SessionRecord::try_from).collect()
    }
}
