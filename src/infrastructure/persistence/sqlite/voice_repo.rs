//! SQLite Voice Repository

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::FromRow;
use std::path::PathBuf;
use uuid::Uuid;

use super::DbPool;
use crate::application::ports::{RepositoryError, VoiceRecord, VoiceRepositoryPort};

/// SQLite Voice Repository
pub struct SqliteVoiceRepository {
    pool: DbPool,
}

impl SqliteVoiceRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
}

#[derive(FromRow)]
struct VoiceRow {
    id: String,
    name: String,
    reference_audio_path: String,
    description: Option<String>,
    created_at: String,
}

impl TryFrom<VoiceRow> for VoiceRecord {
    type Error = RepositoryError;

    fn try_from(row: VoiceRow) -> Result<Self, Self::Error> {
        Ok(VoiceRecord {
            id: Uuid::parse_str(&row.id)
                .map_err(|e| RepositoryError::SerializationError(e.to_string()))?,
            name: row.name,
            reference_audio_path: PathBuf::from(row.reference_audio_path),
            description: row.description,
            created_at: DateTime::parse_from_rfc3339(&row.created_at)
                .map_err(|e| RepositoryError::SerializationError(e.to_string()))?
                .with_timezone(&Utc),
        })
    }
}

#[async_trait]
impl VoiceRepositoryPort for SqliteVoiceRepository {
    async fn save(&self, voice: &VoiceRecord) -> Result<(), RepositoryError> {
        sqlx::query(
            r#"
            INSERT INTO voices (id, name, reference_audio_path, description, created_at)
            VALUES (?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                reference_audio_path = excluded.reference_audio_path,
                description = excluded.description
            "#,
        )
        .bind(voice.id.to_string())
        .bind(&voice.name)
        .bind(voice.reference_audio_path.to_string_lossy().to_string())
        .bind(&voice.description)
        .bind(voice.created_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<VoiceRecord>, RepositoryError> {
        let row: Option<VoiceRow> = sqlx::query_as(
            "SELECT id, name, reference_audio_path, description, created_at FROM voices WHERE id = ?",
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        row.map(VoiceRecord::try_from).transpose()
    }

    async fn find_all(&self) -> Result<Vec<VoiceRecord>, RepositoryError> {
        let rows: Vec<VoiceRow> = sqlx::query_as(
            "SELECT id, name, reference_audio_path, description, created_at FROM voices ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        rows.into_iter().map(VoiceRecord::try_from).collect()
    }

    async fn delete(&self, id: Uuid) -> Result<(), RepositoryError> {
        sqlx::query("DELETE FROM voices WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        Ok(())
    }
}
