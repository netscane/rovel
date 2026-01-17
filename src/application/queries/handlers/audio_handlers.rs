//! Audio Query Handlers - V2 架构

use std::sync::Arc;

use crate::application::error::ApplicationError;
use crate::application::ports::{generate_cache_key, AudioCachePort, NovelRepositoryPort};
use crate::application::queries::audio_queries::{GetAudioQuery, GetAudioResponse};

/// GetAudio Handler - 获取音频数据
pub struct GetAudioHandler {
    audio_cache: Arc<dyn AudioCachePort>,
    novel_repo: Arc<dyn NovelRepositoryPort>,
}

impl GetAudioHandler {
    pub fn new(
        audio_cache: Arc<dyn AudioCachePort>,
        novel_repo: Arc<dyn NovelRepositoryPort>,
    ) -> Self {
        Self {
            audio_cache,
            novel_repo,
        }
    }

    pub async fn handle(&self, query: GetAudioQuery) -> Result<GetAudioResponse, ApplicationError> {
        // 获取片段内容
        let segment = self
            .novel_repo
            .find_segment(query.novel_id, query.segment_index as usize)
            .await?
            .ok_or_else(|| {
                ApplicationError::validation(format!(
                    "Segment not found: {}:{}",
                    query.novel_id, query.segment_index
                ))
            })?;

        // 计算缓存 key
        let cache_key = generate_cache_key(&segment.content, &query.voice_id);

        // 从缓存获取音频
        let audio_data = self
            .audio_cache
            .get(&cache_key)
            .await
            .map_err(|e| ApplicationError::internal(e.to_string()))?
            .ok_or_else(|| {
                ApplicationError::validation(format!(
                    "Audio not found: novel={}, segment={}, voice={}",
                    query.novel_id, query.segment_index, query.voice_id
                ))
            })?;

        Ok(GetAudioResponse {
            audio_data,
            content_type: "audio/wav".to_string(),
        })
    }
}
