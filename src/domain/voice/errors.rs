//! Voice Context - Errors

use thiserror::Error;

use super::VoiceId;

#[derive(Debug, Error)]
pub enum VoiceError {
    #[error("音色不存在: {0}")]
    NotFound(VoiceId),

    #[error("音色已存在: {0}")]
    AlreadyExists(VoiceId),

    #[error("无效的音色名称: {0}")]
    InvalidName(String),

    #[error("无效的参考音频: {0}")]
    InvalidReferenceAudio(String),

    #[error("无效的配置: {0}")]
    InvalidConfig(String),

    #[error("存储错误: {0}")]
    StorageError(String),
}
