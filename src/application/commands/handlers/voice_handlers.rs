//! Voice Command Handlers - V2 架构

use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

use crate::application::commands::{CreateVoice, DeleteVoice};
use crate::application::error::ApplicationError;
use crate::application::ports::{VoiceRecord, VoiceRepositoryPort};

// ============================================================================
// CreateVoice
// ============================================================================

/// 创建音色响应
#[derive(Debug, Clone)]
pub struct CreateVoiceResponse {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
}

/// CreateVoice Handler
pub struct CreateVoiceHandler {
    voice_repo: Arc<dyn VoiceRepositoryPort>,
}

impl CreateVoiceHandler {
    pub fn new(voice_repo: Arc<dyn VoiceRepositoryPort>) -> Self {
        Self { voice_repo }
    }

    pub async fn handle(&self, command: CreateVoice) -> Result<CreateVoiceResponse, ApplicationError> {
        let voice_id = Uuid::new_v4();
        let now = Utc::now();

        let voice = VoiceRecord {
            id: voice_id,
            name: command.name.clone(),
            reference_audio_path: command.reference_audio_path,
            description: command.description.clone(),
            created_at: now,
        };

        self.voice_repo.save(&voice).await?;

        tracing::info!(
            voice_id = %voice_id,
            name = %command.name,
            "Voice created"
        );

        Ok(CreateVoiceResponse {
            id: voice_id,
            name: command.name,
            description: command.description,
        })
    }
}

// ============================================================================
// DeleteVoice
// ============================================================================

/// DeleteVoice Handler
pub struct DeleteVoiceHandler {
    voice_repo: Arc<dyn VoiceRepositoryPort>,
}

impl DeleteVoiceHandler {
    pub fn new(voice_repo: Arc<dyn VoiceRepositoryPort>) -> Self {
        Self { voice_repo }
    }

    pub async fn handle(&self, command: DeleteVoice) -> Result<(), ApplicationError> {
        let voice_id = command.voice_id;

        // 检查音色是否存在
        let voice = self
            .voice_repo
            .find_by_id(voice_id)
            .await?
            .ok_or_else(|| ApplicationError::not_found("Voice", voice_id))?;

        self.voice_repo.delete(voice_id).await?;

        tracing::info!(
            voice_id = %voice_id,
            name = %voice.name,
            "Voice deleted"
        );

        Ok(())
    }
}
