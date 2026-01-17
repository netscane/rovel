//! Voice Context - 音色限界上下文
//!
//! 职责:
//! - 参考音频管理
//! - 音色元数据管理
//! - Voice 查询

mod aggregate;
mod errors;
mod value_objects;

pub use aggregate::Voice;
pub use errors::VoiceError;
pub use value_objects::{AudioFormat, AudioRef, TtsConfig, VoiceId, VoiceName};
