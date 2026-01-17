//! Configuration Types
//!
//! 定义所有配置结构体

use serde::Deserialize;
use std::path::PathBuf;

/// 应用主配置
#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    /// 服务器配置
    #[serde(default)]
    pub server: ServerConfig,

    /// TTS 引擎配置
    #[serde(default)]
    pub tts: TtsConfig,

    /// 数据库配置
    #[serde(default)]
    pub database: DatabaseConfig,

    /// 存储配置
    #[serde(default)]
    pub storage: StorageConfig,

    /// GC 配置
    #[serde(default)]
    pub gc: GcConfig,

    /// 日志配置
    #[serde(default)]
    pub log: LogConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            tts: TtsConfig::default(),
            database: DatabaseConfig::default(),
            storage: StorageConfig::default(),
            gc: GcConfig::default(),
            log: LogConfig::default(),
        }
    }
}

/// 服务器配置
#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    /// 监听地址
    #[serde(default = "default_host")]
    pub host: String,

    /// 监听端口
    #[serde(default = "default_port")]
    pub port: u16,

    /// 公开访问的 Base URL（供外部服务回调使用）
    /// 如果未设置，则使用 http://{host}:{port}
    #[serde(default)]
    pub base_url: Option<String>,
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    5060
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            base_url: None,
        }
    }
}

impl ServerConfig {
    /// 获取服务器地址
    pub fn addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    /// 获取公开的 Base URL
    pub fn public_base_url(&self) -> String {
        self.base_url.clone().unwrap_or_else(|| {
            let host = if self.host == "0.0.0.0" {
                "localhost"
            } else {
                &self.host
            };
            format!("http://{}:{}", host, self.port)
        })
    }
}

/// TTS 引擎配置
#[derive(Debug, Clone, Deserialize)]
pub struct TtsConfig {
    /// TTS 服务基础 URL
    #[serde(default = "default_tts_url")]
    pub url: String,

    /// 请求超时时间（秒）
    #[serde(default = "default_tts_timeout")]
    pub timeout_secs: u64,

    /// 最大重试次数
    #[serde(default)]
    pub max_retries: u32,
}

fn default_tts_url() -> String {
    "http://localhost:8000".to_string()
}

fn default_tts_timeout() -> u64 {
    120
}

impl Default for TtsConfig {
    fn default() -> Self {
        Self {
            url: default_tts_url(),
            timeout_secs: default_tts_timeout(),
            max_retries: 0,
        }
    }
}

/// 数据库配置
#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    /// 数据库文件路径
    #[serde(default = "default_db_path")]
    pub path: String,

    /// 最大连接数
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,
}

fn default_db_path() -> String {
    "data/rovel.db".to_string()
}

fn default_max_connections() -> u32 {
    5
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            path: default_db_path(),
            max_connections: default_max_connections(),
        }
    }
}

impl DatabaseConfig {
    /// 获取数据库 URL
    pub fn database_url(&self) -> String {
        format!("sqlite:{}?mode=rwc", self.path)
    }
}

/// 存储配置
#[derive(Debug, Clone, Deserialize)]
pub struct StorageConfig {
    /// 音频存储目录
    #[serde(default = "default_audio_dir")]
    pub audio_dir: PathBuf,

    /// 小说 TXT 文件存储目录
    #[serde(default = "default_novels_dir")]
    pub novels_dir: PathBuf,

    /// 音色参考音频存储目录
    #[serde(default = "default_voices_dir")]
    pub voices_dir: PathBuf,

    /// 最大存储空间（字节），0 表示不限制
    #[serde(default)]
    pub max_size_bytes: u64,

    /// 上传文件最大大小（字节），默认 10MB
    #[serde(default = "default_max_upload_size")]
    pub max_upload_size: u64,
}

fn default_audio_dir() -> PathBuf {
    PathBuf::from("data/audio")
}

fn default_novels_dir() -> PathBuf {
    PathBuf::from("data/novels")
}

fn default_voices_dir() -> PathBuf {
    PathBuf::from("data/voices")
}

fn default_max_upload_size() -> u64 {
    10 * 1024 * 1024 // 10 MB
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            audio_dir: default_audio_dir(),
            novels_dir: default_novels_dir(),
            voices_dir: default_voices_dir(),
            max_size_bytes: 0,
            max_upload_size: default_max_upload_size(),
        }
    }
}

/// GC（垃圾回收）配置
#[derive(Debug, Clone, Deserialize)]
pub struct GcConfig {
    /// 是否启用自动 GC
    #[serde(default = "default_gc_enabled")]
    pub enabled: bool,

    /// GC 间隔时间（秒）
    #[serde(default = "default_gc_interval")]
    pub interval_secs: u64,

    /// Session 过期时间（秒）
    #[serde(default = "default_session_expire")]
    pub session_expire_secs: u64,

    /// 最大存储空间（字节）
    #[serde(default = "default_max_storage")]
    pub max_storage_bytes: u64,
}

fn default_gc_enabled() -> bool {
    true
}

fn default_gc_interval() -> u64 {
    3600 // 1 小时
}

fn default_session_expire() -> u64 {
    86400 // 24 小时
}

fn default_max_storage() -> u64 {
    10 * 1024 * 1024 * 1024 // 10 GB
}

impl Default for GcConfig {
    fn default() -> Self {
        Self {
            enabled: default_gc_enabled(),
            interval_secs: default_gc_interval(),
            session_expire_secs: default_session_expire(),
            max_storage_bytes: default_max_storage(),
        }
    }
}

/// 日志配置
#[derive(Debug, Clone, Deserialize)]
pub struct LogConfig {
    /// 日志级别
    #[serde(default = "default_log_level")]
    pub level: String,

    /// 是否启用 JSON 格式
    #[serde(default)]
    pub json: bool,
}

fn default_log_level() -> String {
    "info".to_string()
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            json: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.server.port, 5060);
        assert_eq!(config.tts.url, "http://localhost:8000");
        assert_eq!(config.database.path, "data/rovel.db");
    }

    #[test]
    fn test_server_addr() {
        let config = ServerConfig::default();
        assert_eq!(config.addr(), "0.0.0.0:5060");
    }

    #[test]
    fn test_database_url() {
        let config = DatabaseConfig::default();
        assert_eq!(config.database_url(), "sqlite:data/rovel.db?mode=rwc");
    }
}
