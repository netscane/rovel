//! Session Query Handlers

use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

use crate::application::error::ApplicationError;
use crate::application::ports::{
    AudioSegmentRecord, AudioSegmentRepositoryPort, AudioStoragePort, SessionRecord,
    SessionRepositoryPort,
};
use crate::application::queries::{GetPlaybackSession, ListActiveSessions, Query, QueryHandler};

// ============================================================================
// Response DTOs
// ============================================================================

/// 会话详情响应
#[derive(Debug, Clone)]
pub struct SessionResponse {
    pub id: Uuid,
    pub novel_id: Uuid,
    pub voice_id: Uuid,
    pub current_index: usize,
    pub state: String,
    pub window_before: usize,
    pub window_after: usize,
    pub created_at: String,
    pub updated_at: String,
}

impl From<SessionRecord> for SessionResponse {
    fn from(record: SessionRecord) -> Self {
        Self {
            id: record.id,
            novel_id: record.novel_id,
            voice_id: record.voice_id,
            current_index: record.current_index,
            state: record.state.as_str().to_string(),
            window_before: record.window_config.before,
            window_after: record.window_config.after,
            created_at: record.created_at.to_rfc3339(),
            updated_at: record.updated_at.to_rfc3339(),
        }
    }
}

/// 音频段落状态响应
#[derive(Debug, Clone)]
pub struct AudioSegmentStatusResponse {
    pub segment_index: usize,
    pub state: String,
    pub duration_ms: Option<u32>,
    pub error_message: Option<String>,
}

impl From<AudioSegmentRecord> for AudioSegmentStatusResponse {
    fn from(record: AudioSegmentRecord) -> Self {
        Self {
            segment_index: record.segment_index,
            state: record.state.as_str().to_string(),
            duration_ms: record.duration_ms,
            error_message: record.error_message,
        }
    }
}

/// 会话段落响应
#[derive(Debug, Clone)]
pub struct SessionSegmentsResponse {
    pub session_id: Uuid,
    pub segments: Vec<AudioSegmentStatusResponse>,
}

// ============================================================================
// GetPlaybackSession Query
// ============================================================================

impl Query for GetPlaybackSession {}

/// GetPlaybackSession Handler
pub struct GetSessionHandler<S: SessionRepositoryPort> {
    session_repo: Arc<S>,
}

impl<S: SessionRepositoryPort> GetSessionHandler<S> {
    pub fn new(session_repo: Arc<S>) -> Self {
        Self { session_repo }
    }
}

#[async_trait]
impl<S: SessionRepositoryPort + 'static> QueryHandler<GetPlaybackSession> for GetSessionHandler<S> {
    type Output = SessionResponse;
    type Error = ApplicationError;

    async fn handle(&self, query: GetPlaybackSession) -> Result<Self::Output, Self::Error> {
        let session_id = *query.session_id.as_uuid();

        let session = self
            .session_repo
            .find_by_id(session_id)
            .await?
            .ok_or_else(|| ApplicationError::not_found("Session", session_id))?;

        Ok(SessionResponse::from(session))
    }
}

// ============================================================================
// ListSessions Query
// ============================================================================

/// 列出所有会话查询
#[derive(Debug, Clone)]
pub struct ListSessions;

impl Query for ListSessions {}

/// ListSessions Handler
pub struct ListSessionsHandler<S: SessionRepositoryPort> {
    session_repo: Arc<S>,
}

impl<S: SessionRepositoryPort> ListSessionsHandler<S> {
    pub fn new(session_repo: Arc<S>) -> Self {
        Self { session_repo }
    }
}

#[async_trait]
impl<S: SessionRepositoryPort + 'static> QueryHandler<ListSessions> for ListSessionsHandler<S> {
    type Output = Vec<SessionResponse>;
    type Error = ApplicationError;

    async fn handle(&self, _query: ListSessions) -> Result<Self::Output, Self::Error> {
        let sessions = self.session_repo.find_all().await?;
        Ok(sessions.into_iter().map(SessionResponse::from).collect())
    }
}

// ============================================================================
// ListActiveSessions Query
// ============================================================================

impl Query for ListActiveSessions {}

/// ListActiveSessions Handler
pub struct ListActiveSessionsHandler<S: SessionRepositoryPort> {
    session_repo: Arc<S>,
}

impl<S: SessionRepositoryPort> ListActiveSessionsHandler<S> {
    pub fn new(session_repo: Arc<S>) -> Self {
        Self { session_repo }
    }
}

#[async_trait]
impl<S: SessionRepositoryPort + 'static> QueryHandler<ListActiveSessions>
    for ListActiveSessionsHandler<S>
{
    type Output = Vec<SessionResponse>;
    type Error = ApplicationError;

    async fn handle(&self, _query: ListActiveSessions) -> Result<Self::Output, Self::Error> {
        let sessions = self.session_repo.find_active().await?;
        Ok(sessions.into_iter().map(SessionResponse::from).collect())
    }
}

// ============================================================================
// GetSessionSegments Query
// ============================================================================

/// 获取会话段落状态查询
#[derive(Debug, Clone)]
pub struct GetSessionSegments {
    pub session_id: Uuid,
}

impl Query for GetSessionSegments {}

/// GetSessionSegments Handler
pub struct GetSessionSegmentsHandler<S, A>
where
    S: SessionRepositoryPort,
    A: AudioSegmentRepositoryPort,
{
    session_repo: Arc<S>,
    audio_segment_repo: Arc<A>,
}

impl<S, A> GetSessionSegmentsHandler<S, A>
where
    S: SessionRepositoryPort,
    A: AudioSegmentRepositoryPort,
{
    pub fn new(session_repo: Arc<S>, audio_segment_repo: Arc<A>) -> Self {
        Self {
            session_repo,
            audio_segment_repo,
        }
    }
}

#[async_trait]
impl<S, A> QueryHandler<GetSessionSegments> for GetSessionSegmentsHandler<S, A>
where
    S: SessionRepositoryPort + 'static,
    A: AudioSegmentRepositoryPort + 'static,
{
    type Output = SessionSegmentsResponse;
    type Error = ApplicationError;

    async fn handle(&self, query: GetSessionSegments) -> Result<Self::Output, Self::Error> {
        // 验证会话存在
        self.session_repo
            .find_by_id(query.session_id)
            .await?
            .ok_or_else(|| ApplicationError::not_found("Session", query.session_id))?;

        let segments = self
            .audio_segment_repo
            .find_by_session(query.session_id)
            .await?;

        Ok(SessionSegmentsResponse {
            session_id: query.session_id,
            segments: segments
                .into_iter()
                .map(AudioSegmentStatusResponse::from)
                .collect(),
        })
    }
}

// ============================================================================
// GetAudio Query
// ============================================================================

/// 获取音频查询
#[derive(Debug, Clone)]
pub struct GetAudio {
    pub session_id: Uuid,
    pub segment_index: usize,
}

impl Query for GetAudio {}

/// 音频响应
#[derive(Debug)]
pub enum AudioResponse {
    /// 音频已就绪
    Ready {
        data: Vec<u8>,
        duration_ms: Option<u32>,
    },
    /// 正在推理中
    Inferring,
    /// 推理失败
    Failed { error_message: String },
    /// 未找到
    NotFound,
}

/// GetAudio Handler
pub struct GetAudioHandler<S, A, AS>
where
    S: SessionRepositoryPort,
    A: AudioSegmentRepositoryPort,
    AS: AudioStoragePort,
{
    session_repo: Arc<S>,
    audio_segment_repo: Arc<A>,
    audio_storage: Arc<AS>,
}

impl<S, A, AS> GetAudioHandler<S, A, AS>
where
    S: SessionRepositoryPort,
    A: AudioSegmentRepositoryPort,
    AS: AudioStoragePort,
{
    pub fn new(session_repo: Arc<S>, audio_segment_repo: Arc<A>, audio_storage: Arc<AS>) -> Self {
        Self {
            session_repo,
            audio_segment_repo,
            audio_storage,
        }
    }
}

#[async_trait]
impl<S, A, AS> QueryHandler<GetAudio> for GetAudioHandler<S, A, AS>
where
    S: SessionRepositoryPort + 'static,
    A: AudioSegmentRepositoryPort + 'static,
    AS: AudioStoragePort + 'static,
{
    type Output = AudioResponse;
    type Error = ApplicationError;

    async fn handle(&self, query: GetAudio) -> Result<Self::Output, Self::Error> {
        use crate::application::ports::AudioSegmentState;

        // 验证会话存在
        self.session_repo
            .find_by_id(query.session_id)
            .await?
            .ok_or_else(|| ApplicationError::not_found("Session", query.session_id))?;

        // 查找音频段落
        let segment = self
            .audio_segment_repo
            .find_by_session_and_index(query.session_id, query.segment_index)
            .await?;

        match segment {
            None => Ok(AudioResponse::NotFound),
            Some(seg) => match seg.state {
                AudioSegmentState::Ready => {
                    // 读取音频数据
                    let data = self
                        .audio_storage
                        .read_audio(query.session_id, query.segment_index)
                        .await
                        .map_err(|e| ApplicationError::StorageError(e.to_string()))?;

                    // 更新访问时间
                    let _ = self.audio_segment_repo.touch(seg.id).await;

                    Ok(AudioResponse::Ready {
                        data,
                        duration_ms: seg.duration_ms,
                    })
                }
                AudioSegmentState::Inferring | AudioSegmentState::Pending => {
                    Ok(AudioResponse::Inferring)
                }
                AudioSegmentState::Failed => Ok(AudioResponse::Failed {
                    error_message: seg.error_message.unwrap_or_else(|| "Unknown error".to_string()),
                }),
            },
        }
    }
}
