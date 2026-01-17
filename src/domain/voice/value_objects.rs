//! Voice Context - Value Objects

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

/// 音色唯一标识
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VoiceId(Uuid);

impl VoiceId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for VoiceId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for VoiceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// 音色名称
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoiceName(String);

impl VoiceName {
    pub fn new(name: impl Into<String>) -> Result<Self, &'static str> {
        let name = name.into();
        if name.is_empty() {
            return Err("音色名称不能为空");
        }
        if name.len() > 100 {
            return Err("音色名称长度不能超过100字符");
        }
        Ok(Self(name))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for VoiceName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// 音频格式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AudioFormat {
    Wav,
    Mp3,
    Flac,
    Ogg,
}

impl AudioFormat {
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "wav" => Some(Self::Wav),
            "mp3" => Some(Self::Mp3),
            "flac" => Some(Self::Flac),
            "ogg" => Some(Self::Ogg),
            _ => None,
        }
    }

    pub fn extension(&self) -> &'static str {
        match self {
            Self::Wav => "wav",
            Self::Mp3 => "mp3",
            Self::Flac => "flac",
            Self::Ogg => "ogg",
        }
    }
}

/// 音频引用 - 参考音频的路径和格式
///
/// 不变量:
/// - path 必须指向有效文件
/// - format 必须与文件实际格式匹配
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioRef {
    path: PathBuf,
    format: AudioFormat,
}

impl AudioRef {
    pub fn new(path: PathBuf, format: AudioFormat) -> Self {
        Self { path, format }
    }

    /// 从路径自动推断格式
    pub fn from_path(path: PathBuf) -> Result<Self, &'static str> {
        let format = path
            .extension()
            .and_then(|e| e.to_str())
            .and_then(AudioFormat::from_extension)
            .ok_or("无法识别的音频格式")?;

        Ok(Self { path, format })
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    pub fn format(&self) -> AudioFormat {
        self.format
    }
}

/// TTS 配置参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsConfig {
    /// 语速 (0.5 - 2.0)
    pub speed: f32,
    /// 音调 (-12 - 12)
    pub pitch: i8,
    /// 音量 (0.0 - 1.0)
    pub volume: f32,
}

impl Default for TtsConfig {
    fn default() -> Self {
        Self {
            speed: 1.0,
            pitch: 0,
            volume: 1.0,
        }
    }
}

impl TtsConfig {
    pub fn validate(&self) -> Result<(), &'static str> {
        if !(0.5..=2.0).contains(&self.speed) {
            return Err("语速必须在 0.5 到 2.0 之间");
        }
        if !(-12..=12).contains(&self.pitch) {
            return Err("音调必须在 -12 到 12 之间");
        }
        if !(0.0..=1.0).contains(&self.volume) {
            return Err("音量必须在 0.0 到 1.0 之间");
        }
        Ok(())
    }
}
