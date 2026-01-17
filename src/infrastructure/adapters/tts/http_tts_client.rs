//! HTTP TTS Client - 调用外部 TTS HTTP 服务
//!
//! 实现 TtsEnginePort trait，通过 HTTP 调用外部 TTS 服务
//!
//! 外部 TTS API:
//! POST http://localhost:8000/api/tts/infer
//! Request: {"text": "...", "voice_ref": "http://..."}  (JSON)
//! Response: audio/wav binary, metadata in headers

use async_trait::async_trait;
use reqwest::Client;
use serde::Serialize;
use std::time::Duration;

use crate::application::ports::{InferRequest, InferResponse, TtsEnginePort, TtsError};

/// TTS 推理请求体 (JSON)
#[derive(Debug, Serialize)]
struct TtsHttpRequest {
    /// 要合成的文本
    text: String,
    /// 参考音频的 URL 或路径（TTS 服务自行下载/读取并缓存）
    voice_ref: String,
}

/// HTTP TTS 客户端配置
#[derive(Debug, Clone)]
pub struct HttpTtsClientConfig {
    /// TTS 服务基础 URL
    pub base_url: String,
    /// 请求超时时间（秒）
    pub timeout_secs: u64,
    /// 重试次数
    pub max_retries: u32,
}

impl Default for HttpTtsClientConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:8000".to_string(),
            timeout_secs: 120,
            max_retries: 0,
        }
    }
}

impl HttpTtsClientConfig {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            ..Default::default()
        }
    }

    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }
}

/// HTTP TTS 客户端
///
/// 通过 HTTP 调用外部 TTS 服务
pub struct HttpTtsClient {
    client: Client,
    config: HttpTtsClientConfig,
}

impl HttpTtsClient {
    /// 创建新的 HTTP TTS 客户端
    pub fn new(config: HttpTtsClientConfig) -> Result<Self, TtsError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .build()
            .map_err(|e| TtsError::NetworkError(e.to_string()))?;

        Ok(Self { client, config })
    }

    /// 使用默认配置创建客户端
    pub fn with_default_config() -> Result<Self, TtsError> {
        Self::new(HttpTtsClientConfig::default())
    }

    /// 获取推理 URL
    fn infer_url(&self) -> String {
        format!("{}/api/tts/infer", self.config.base_url)
    }

    /// 获取健康检查 URL
    fn health_url(&self) -> String {
        format!("{}/health", self.config.base_url)
    }
}

#[async_trait]
impl TtsEnginePort for HttpTtsClient {
    async fn infer(&self, request: InferRequest) -> Result<InferResponse, TtsError> {
        let http_request = TtsHttpRequest {
            text: request.text.clone(),
            voice_ref: request.voice_ref.clone(),
        };

        tracing::debug!(
            url = %self.infer_url(),
            text_len = http_request.text.len(),
            voice_ref = %http_request.voice_ref,
            "Sending TTS infer request"
        );

        let response = self
            .client
            .post(&self.infer_url())
            .json(&http_request)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    TtsError::Timeout
                } else if e.is_connect() {
                    TtsError::NetworkError(format!("Cannot connect to TTS service: {}", e))
                } else {
                    TtsError::NetworkError(e.to_string())
                }
            })?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(TtsError::ServiceError(format!(
                "HTTP {}: {}",
                status, error_text
            )));
        }

        // 从 headers 提取元数据
        let headers = response.headers();
        let session_id = headers
            .get("X-TTS-Session-Id")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown")
            .to_string();
        let duration_ms = headers
            .get("X-TTS-Duration-Ms")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse().ok());
        let sample_rate = headers
            .get("X-TTS-Sample-Rate")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse().ok());

        // 直接获取音频字节
        let audio_data = response
            .bytes()
            .await
            .map_err(|e| TtsError::InvalidResponse(format!("Failed to read audio: {}", e)))?
            .to_vec();

        tracing::info!(
            session_id = %session_id,
            duration_ms = ?duration_ms,
            sample_rate = ?sample_rate,
            audio_size = audio_data.len(),
            "TTS inference completed"
        );

        Ok(InferResponse {
            session_id,
            audio_data,
            duration_ms,
            sample_rate,
        })
    }

    async fn health_check(&self) -> bool {
        match self
            .client
            .get(&self.health_url())
            .timeout(Duration::from_secs(5))
            .send()
            .await
        {
            Ok(response) => response.status().is_success(),
            Err(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = HttpTtsClientConfig::default();
        assert_eq!(config.base_url, "http://localhost:8000");
        assert_eq!(config.timeout_secs, 120);
    }

    #[test]
    fn test_config_builder() {
        let config = HttpTtsClientConfig::new("http://example.com:9000").with_timeout(60);
        assert_eq!(config.base_url, "http://example.com:9000");
        assert_eq!(config.timeout_secs, 60);
    }
}
