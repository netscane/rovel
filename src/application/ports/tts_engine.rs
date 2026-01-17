//! TTS Engine Port - TTS 推理引擎抽象
//!
//! 定义 TTS 推理的抽象接口，具体实现在 infrastructure/adapters 层

use async_trait::async_trait;
use thiserror::Error;

/// TTS 错误
#[derive(Debug, Error)]
pub enum TtsError {
    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Request timeout")]
    Timeout,

    #[error("Service error: {0}")]
    ServiceError(String),

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("Voice not found: {0}")]
    VoiceNotFound(String),
}

/// TTS 推理请求
#[derive(Debug, Clone)]
pub struct InferRequest {
    /// 要合成的文本内容
    pub text: String,
    /// 参考音频的 URL 或路径（TTS 服务会自行下载/读取并缓存）
    pub voice_ref: String,
    /// 音色 ID（用于日志和追踪）
    pub voice_id: String,
}

/// TTS 推理响应
#[derive(Debug, Clone)]
pub struct InferResponse {
    /// TTS 服务会话 ID（用于追踪）
    pub session_id: String,
    /// 原始音频数据（WAV/PCM）
    pub audio_data: Vec<u8>,
    /// 音频时长（毫秒）
    pub duration_ms: Option<u64>,
    /// 采样率
    pub sample_rate: Option<u32>,
}

/// TTS Engine Port
///
/// 外部 TTS 服务的抽象接口
#[async_trait]
pub trait TtsEnginePort: Send + Sync {
    /// 执行 TTS 推理
    ///
    /// 发送文本和参考音频到外部 TTS 服务，返回合成的音频数据
    async fn infer(&self, request: InferRequest) -> Result<InferResponse, TtsError>;

    /// 检查 TTS 服务是否可用
    async fn health_check(&self) -> bool {
        true // 默认实现
    }
}
