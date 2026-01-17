//! Application State
//!
//! V2 架构 - 基于 ARCHITECTURE.md 设计
//! 包含所有 Command/Query Handlers 的应用状态

use std::sync::Arc;

use crate::application::{
    // Command handlers
    ChangeVoiceHandler, CloseSessionHandler, CreateNovelFromTextHandler, CreateVoiceHandler,
    DeleteNovelHandler, DeleteVoiceHandler, PlayHandler, ProcessNovelSegmentsHandler,
    QueryTaskStatusHandler, SeekHandler, SubmitInferHandler,
    // Query handlers
    GetAudioHandler, GetNovelHandler, GetNovelSegmentsHandler, GetVoiceHandler,
    ListNovelsHandler, ListVoicesHandler,
    // Ports
    AudioCachePort, NovelRepositoryPort, SessionManagerPort, TaskManagerPort, TtsEnginePort,
    VoiceRepositoryPort,
};
use crate::infrastructure::events::EventPublisher;

/// 应用状态
///
/// V2 架构：SessionManager 和 TaskManager 为内存实现
pub struct AppState {
    // ========== Ports ==========
    pub session_manager: Arc<dyn SessionManagerPort>,
    pub task_manager: Arc<dyn TaskManagerPort>,
    pub novel_repo: Arc<dyn NovelRepositoryPort>,
    pub voice_repo: Arc<dyn VoiceRepositoryPort>,
    pub audio_cache: Arc<dyn AudioCachePort>,
    pub tts_engine: Arc<dyn TtsEnginePort>,
    pub event_publisher: Arc<EventPublisher>,

    // ========== Command Handlers ==========
    pub create_novel_handler: CreateNovelFromTextHandler,
    pub process_novel_handler: ProcessNovelSegmentsHandler,
    pub delete_novel_handler: DeleteNovelHandler,
    pub create_voice_handler: CreateVoiceHandler,
    pub delete_voice_handler: DeleteVoiceHandler,
    pub play_handler: PlayHandler,
    pub seek_handler: SeekHandler,
    pub change_voice_handler: ChangeVoiceHandler,
    pub close_session_handler: CloseSessionHandler,
    pub submit_infer_handler: SubmitInferHandler,
    pub query_task_status_handler: QueryTaskStatusHandler,

    // ========== Query Handlers ==========
    pub get_novel_handler: GetNovelHandler,
    pub list_novels_handler: ListNovelsHandler,
    pub get_novel_segments_handler: GetNovelSegmentsHandler,
    pub get_voice_handler: GetVoiceHandler,
    pub list_voices_handler: ListVoicesHandler,
    pub get_audio_handler: GetAudioHandler,
}

impl AppState {
    /// 创建应用状态
    pub fn new(
        session_manager: Arc<dyn SessionManagerPort>,
        task_manager: Arc<dyn TaskManagerPort>,
        novel_repo: Arc<dyn NovelRepositoryPort>,
        voice_repo: Arc<dyn VoiceRepositoryPort>,
        audio_cache: Arc<dyn AudioCachePort>,
        tts_engine: Arc<dyn TtsEnginePort>,
        event_publisher: Arc<EventPublisher>,
    ) -> Self {
        Self {
            // Ports
            session_manager: session_manager.clone(),
            task_manager: task_manager.clone(),
            novel_repo: novel_repo.clone(),
            voice_repo: voice_repo.clone(),
            audio_cache: audio_cache.clone(),
            tts_engine: tts_engine.clone(),
            event_publisher: event_publisher.clone(),

            // Command handlers
            create_novel_handler: CreateNovelFromTextHandler::new(novel_repo.clone()),
            process_novel_handler: ProcessNovelSegmentsHandler::new(novel_repo.clone()),
            delete_novel_handler: DeleteNovelHandler::new(novel_repo.clone()),
            create_voice_handler: CreateVoiceHandler::new(voice_repo.clone()),
            delete_voice_handler: DeleteVoiceHandler::new(voice_repo.clone()),
            play_handler: PlayHandler::new(
                session_manager.clone(),
                task_manager.clone(),
                novel_repo.clone(),
                voice_repo.clone(),
            ),
            seek_handler: SeekHandler::new(session_manager.clone(), task_manager.clone()),
            change_voice_handler: ChangeVoiceHandler::new(
                session_manager.clone(),
                task_manager.clone(),
                voice_repo.clone(),
            ),
            close_session_handler: CloseSessionHandler::new(
                session_manager.clone(),
                task_manager.clone(),
                event_publisher.clone(),
            ),
            submit_infer_handler: SubmitInferHandler::new(
                session_manager.clone(),
                task_manager.clone(),
                novel_repo.clone(),
                audio_cache.clone(),
            ),
            query_task_status_handler: QueryTaskStatusHandler::new(task_manager.clone()),

            // Query handlers
            get_novel_handler: GetNovelHandler::new(novel_repo.clone()),
            list_novels_handler: ListNovelsHandler::new(novel_repo.clone()),
            get_novel_segments_handler: GetNovelSegmentsHandler::new(novel_repo.clone()),
            get_voice_handler: GetVoiceHandler::new(voice_repo.clone()),
            list_voices_handler: ListVoicesHandler::new(voice_repo.clone()),
            get_audio_handler: GetAudioHandler::new(audio_cache.clone(), novel_repo.clone()),
        }
    }
}
