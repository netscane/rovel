//! Application Ports - 出站端口定义
//!
//! 定义应用层与基础设施层的抽象接口

mod audio_cache;
mod audio_storage;
mod audio_transcoder;
mod repositories;
mod session_manager;
mod task_manager;
mod text_segmenter;
mod tts_engine;

pub use audio_cache::{
    generate_cache_key, AudioCachePort, CacheEntry, CacheError, CacheMetadata, CacheStats,
};
pub use audio_storage::{
    AudioStorageError, AudioStoragePort, GcConfig, GcResult, StorageStats,
};
pub use repositories::{
    AudioSegmentRecord, AudioSegmentRepositoryPort, AudioSegmentState, NovelRecord,
    NovelRepositoryPort, NovelStatus, RepositoryError, SessionRecord, SessionRepositoryPort,
    SessionState, TextSegmentRecord, VoiceRecord, VoiceRepositoryPort, WindowConfig,
};
pub use session_manager::{Session, SessionError, SessionManagerPort};
pub use task_manager::{InferenceTask, TaskError, TaskManagerPort, TaskState};
pub use text_segmenter::{SegmentConfig, SegmentedText, TextSegmenterPort};
pub use tts_engine::{InferRequest, InferResponse, TtsEnginePort, TtsError};
pub use audio_transcoder::{
    AudioFormat, AudioInfo, AudioTranscoderPort, TranscodeConfig, TranscodeError, TranscodeResult,
};
