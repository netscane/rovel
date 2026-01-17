//! Voice Context - Aggregate Root

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{AudioRef, TtsConfig, VoiceId, VoiceName};

/// Voice 聚合根
///
/// 不变量:
/// - Voice 必须有且只有一个 reference audio
/// - reference audio 不可被播放上下文修改
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Voice {
    id: VoiceId,
    name: VoiceName,
    reference_audio: AudioRef,
    config: TtsConfig,
    description: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl Voice {
    /// 创建新音色
    pub fn new(name: VoiceName, reference_audio: AudioRef) -> Self {
        let now = Utc::now();
        Self {
            id: VoiceId::new(),
            name,
            reference_audio,
            config: TtsConfig::default(),
            description: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// 创建带配置的音色
    pub fn with_config(name: VoiceName, reference_audio: AudioRef, config: TtsConfig) -> Self {
        let mut voice = Self::new(name, reference_audio);
        voice.config = config;
        voice
    }

    /// 更新 TTS 配置
    pub fn update_config(&mut self, config: TtsConfig) -> Result<(), &'static str> {
        config.validate()?;
        self.config = config;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// 更新音色名称
    pub fn rename(&mut self, name: VoiceName) {
        self.name = name;
        self.updated_at = Utc::now();
    }

    /// 设置描述
    pub fn set_description(&mut self, description: Option<String>) {
        self.description = description;
        self.updated_at = Utc::now();
    }

    // Getters
    pub fn id(&self) -> &VoiceId {
        &self.id
    }

    pub fn name(&self) -> &VoiceName {
        &self.name
    }

    pub fn reference_audio(&self) -> &AudioRef {
        &self.reference_audio
    }

    pub fn config(&self) -> &TtsConfig {
        &self.config
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_voice_creation() {
        let name = VoiceName::new("测试音色").unwrap();
        let audio = AudioRef::from_path(PathBuf::from("/tmp/ref.wav")).unwrap();
        let voice = Voice::new(name, audio);

        assert_eq!(voice.name().as_str(), "测试音色");
        assert_eq!(voice.config().speed, 1.0);
    }

    #[test]
    fn test_config_validation() {
        let config = TtsConfig {
            speed: 3.0, // 超出范围
            pitch: 0,
            volume: 1.0,
        };
        assert!(config.validate().is_err());
    }
}
