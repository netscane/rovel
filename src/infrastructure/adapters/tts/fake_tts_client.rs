//! Fake TTS Client - 用于测试的 TTS 客户端
//!
//! 始终返回固定的音频文件，不实际调用 TTS 服务

use async_trait::async_trait;
use std::path::PathBuf;

use crate::application::ports::{InferRequest, InferResponse, TtsEnginePort, TtsError};

/// Fake TTS Client 配置
#[derive(Debug, Clone)]
pub struct FakeTtsClientConfig {
    /// 固定返回的音频文件路径
    pub audio_file_path: PathBuf,
    /// 固定返回的音频时长（毫秒）
    pub duration_ms: u64,
    /// 采样率
    pub sample_rate: u32,
}

impl Default for FakeTtsClientConfig {
    fn default() -> Self {
        Self {
            audio_file_path: PathBuf::from("/home/github/rovel/Speaker_1.wav"),
            duration_ms: 5000,
            sample_rate: 22050,
        }
    }
}

/// Fake TTS Client
///
/// 用于测试，始终返回配置的固定音频文件
pub struct FakeTtsClient {
    config: FakeTtsClientConfig,
    /// 缓存的音频数据
    audio_data: Vec<u8>,
}

impl FakeTtsClient {
    /// 创建新的 FakeTtsClient
    pub fn new(config: FakeTtsClientConfig) -> Result<Self, std::io::Error> {
        let audio_data = std::fs::read(&config.audio_file_path)?;
        tracing::info!(
            path = %config.audio_file_path.display(),
            duration_ms = config.duration_ms,
            "FakeTtsClient initialized"
        );
        Ok(Self { config, audio_data })
    }

    /// 使用默认配置创建
    pub fn with_defaults() -> Result<Self, std::io::Error> {
        Self::new(FakeTtsClientConfig::default())
    }
}

#[async_trait]
impl TtsEnginePort for FakeTtsClient {
    async fn infer(&self, request: InferRequest) -> Result<InferResponse, TtsError> {
        tracing::debug!(
            text_len = request.text.len(),
            voice_id = %request.voice_id,
            voice_ref = %request.voice_ref,
            "FakeTtsClient: returning fixed audio"
        );

        // 模拟推理延迟
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        Ok(InferResponse {
            session_id: format!("fake-{}", uuid::Uuid::new_v4()),
            audio_data: self.audio_data.clone(),
            duration_ms: Some(self.config.duration_ms),
            sample_rate: Some(self.config.sample_rate),
        })
    }

    async fn health_check(&self) -> bool {
        true
    }
}
