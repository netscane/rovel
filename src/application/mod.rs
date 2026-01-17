//! 应用层 - 用例编排
//!
//! 包含：
//! - ports: 六边形架构端口定义（TtsEngine、Repository、SessionManager、TaskManager 等）
//! - commands: CQRS 命令及处理器
//! - queries: CQRS 查询及处理器
//! - error: 应用层错误定义

pub mod commands;
pub mod error;
pub mod ports;
pub mod queries;

// Re-exports
pub use commands::{
    // Infer commands
    QueryTaskStatusCommand,
    QueryTaskStatusResponse,
    SubmitInferCommand,
    SubmitInferResponse,
    TaskInfo,
    TaskStatusInfo,
    // Novel commands
    CreateNovel,
    CreateNovelFromText,
    DeleteNovel,
    ProcessNovelSegments,
    // Session commands
    ChangeVoiceCommand,
    ChangeVoiceResponse,
    CloseSessionCommand,
    CloseSessionResponse,
    PlayCommand,
    PlayResponse,
    SeekCommand,
    SeekResponse,
    // Voice commands
    CreateVoice,
    DeleteVoice,
    // Handlers
    handlers::{
        ChangeVoiceHandler, CloseSessionHandler, CreateNovelFromTextHandler, CreateVoiceHandler,
        DeleteNovelHandler, DeleteVoiceHandler, PlayHandler, ProcessNovelSegmentsHandler,
        QueryTaskStatusHandler, SeekHandler, SubmitInferHandler,
    },
};

pub use error::ApplicationError;

pub use ports::{
    // Audio cache
    generate_cache_key,
    AudioCachePort,
    CacheEntry,
    CacheError,
    CacheMetadata,
    CacheStats,
    // Repositories
    AudioSegmentRecord,
    AudioSegmentRepositoryPort,
    NovelRecord,
    NovelRepositoryPort,
    NovelStatus,
    RepositoryError,
    TextSegmentRecord,
    VoiceRecord,
    VoiceRepositoryPort,
    // Session manager
    Session,
    SessionError,
    SessionManagerPort,
    // Task manager
    InferenceTask,
    TaskError,
    TaskManagerPort,
    TaskState,
    // Text segmenter
    SegmentConfig,
    SegmentedText,
    TextSegmenterPort,
    // TTS engine
    InferRequest,
    InferResponse,
    TtsEnginePort,
    TtsError,
};

pub use queries::{
    // Audio queries
    GetAudioQuery,
    GetAudioResponse,
    // Novel queries
    GetNovel,
    GetNovelSegments,
    ListNovels,
    // Voice queries
    GetVoice,
    ListVoices,
    // Handlers
    handlers::{GetAudioHandler, GetNovelHandler, GetNovelSegmentsHandler, GetVoiceHandler, ListNovelsHandler, ListVoicesHandler},
};
