//! Audio Transcoder Port - 音频转码抽象
//!
//! 定义音频转码的抽象接口，支持将 WAV 转换为其他格式（如 Opus、AAC）

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 转码错误
#[derive(Debug, Error)]
pub enum TranscodeError {
    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Encoding error: {0}")]
    EncodingError(String),

    #[error("Decoding error: {0}")]
    DecodingError(String),

    #[error("IO error: {0}")]
    IoError(String),
}

/// 音频输出格式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum AudioFormat {
    /// 原始 WAV，不转码
    #[default]
    Wav,
    /// Opus 格式 - 推荐用于 WebRTC/实时通话
    Opus,
    /// MP3 格式 - 通用兼容
    Mp3,
}

impl std::fmt::Display for AudioFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AudioFormat::Wav => write!(f, "wav"),
            AudioFormat::Opus => write!(f, "opus"),
            AudioFormat::Mp3 => write!(f, "mp3"),
        }
    }
}

impl std::str::FromStr for AudioFormat {
    type Err = TranscodeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "wav" => Ok(AudioFormat::Wav),
            "opus" => Ok(AudioFormat::Opus),
            "mp3" => Ok(AudioFormat::Mp3),
            _ => Err(TranscodeError::UnsupportedFormat(s.to_string())),
        }
    }
}

/// 转码配置
#[derive(Debug, Clone)]
pub struct TranscodeConfig {
    /// 输出格式
    pub format: AudioFormat,
    /// 目标比特率（bps），用于有损压缩格式
    /// Opus 推荐: 16000-64000 用于语音
    pub bitrate: Option<u32>,
    /// 目标采样率（Hz）
    /// 如果为 None，则保持原始采样率
    pub sample_rate: Option<u32>,
    /// 声道数
    /// 如果为 None，则保持原始声道数
    pub channels: Option<u8>,
}

impl Default for TranscodeConfig {
    fn default() -> Self {
        Self {
            format: AudioFormat::Wav,
            bitrate: Some(32000), // 32kbps，语音足够
            sample_rate: None,    // 保持原始
            channels: Some(1),    // 单声道
        }
    }
}

/// 转码结果
#[derive(Debug, Clone)]
pub struct TranscodeResult {
    /// 转码后的音频数据
    pub audio_data: Vec<u8>,
    /// 输出格式
    pub format: AudioFormat,
    /// 时长（毫秒）
    pub duration_ms: u64,
    /// 采样率
    pub sample_rate: u32,
    /// 声道数
    pub channels: u8,
    /// 原始大小（字节）
    pub original_size: usize,
    /// 转码后大小（字节）
    pub transcoded_size: usize,
}

/// Audio Transcoder Port
///
/// 音频转码的抽象接口
#[async_trait]
pub trait AudioTranscoderPort: Send + Sync {
    /// 转码音频
    ///
    /// # Arguments
    /// * `wav_data` - 输入的 WAV 音频数据
    /// * `config` - 转码配置
    ///
    /// # Returns
    /// 转码后的音频数据和元信息
    async fn transcode(
        &self,
        wav_data: &[u8],
        config: &TranscodeConfig,
    ) -> Result<TranscodeResult, TranscodeError>;

    /// 获取音频信息（不转码）
    fn get_audio_info(&self, wav_data: &[u8]) -> Result<AudioInfo, TranscodeError>;

    /// 检查是否支持指定格式
    fn supports_format(&self, format: AudioFormat) -> bool;
}

/// 音频信息
#[derive(Debug, Clone)]
pub struct AudioInfo {
    /// 时长（毫秒）
    pub duration_ms: u64,
    /// 采样率
    pub sample_rate: u32,
    /// 声道数
    pub channels: u8,
    /// 位深度
    pub bits_per_sample: u16,
    /// 数据大小（字节）
    pub data_size: usize,
}
