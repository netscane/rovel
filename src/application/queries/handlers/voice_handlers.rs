//! Voice Query Handlers - V2 架构

use std::sync::Arc;
use uuid::Uuid;

use crate::application::error::ApplicationError;
use crate::application::ports::{VoiceRecord, VoiceRepositoryPort};
use crate::application::queries::{GetVoice, ListVoices};

// ============================================================================
// Response DTOs
// ============================================================================

/// 音色详情响应
#[derive(Debug, Clone)]
pub struct VoiceResponse {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub created_at: String,
}

impl From<VoiceRecord> for VoiceResponse {
    fn from(record: VoiceRecord) -> Self {
        Self {
            id: record.id,
            name: record.name,
            description: record.description,
            created_at: record.created_at.to_rfc3339(),
        }
    }
}

// ============================================================================
// Handlers
// ============================================================================

/// GetVoice Handler
pub struct GetVoiceHandler {
    voice_repo: Arc<dyn VoiceRepositoryPort>,
}

impl GetVoiceHandler {
    pub fn new(voice_repo: Arc<dyn VoiceRepositoryPort>) -> Self {
        Self { voice_repo }
    }

    pub async fn handle(&self, query: GetVoice) -> Result<VoiceResponse, ApplicationError> {
        let voice = self
            .voice_repo
            .find_by_id(query.voice_id)
            .await?
            .ok_or_else(|| ApplicationError::not_found("Voice", query.voice_id))?;

        Ok(VoiceResponse::from(voice))
    }
}

/// ListVoices Handler
pub struct ListVoicesHandler {
    voice_repo: Arc<dyn VoiceRepositoryPort>,
}

impl ListVoicesHandler {
    pub fn new(voice_repo: Arc<dyn VoiceRepositoryPort>) -> Self {
        Self { voice_repo }
    }

    pub async fn handle(&self, _query: ListVoices) -> Result<Vec<VoiceResponse>, ApplicationError> {
        let voices = self.voice_repo.find_all().await?;
        Ok(voices.into_iter().map(VoiceResponse::from).collect())
    }
}
