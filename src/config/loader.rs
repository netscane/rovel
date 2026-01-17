//! Configuration Loader
//!
//! 实现多源配置加载与合并逻辑
//!
//! 优先级（从高到低）：
//! 1. 环境变量
//! 2. 配置文件（config.toml）
//! 3. 默认值

use config::{Config, ConfigError as ConfigCrateError, Environment, File};
use std::path::Path;
use thiserror::Error;

use super::types::AppConfig;

/// 配置加载错误
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Failed to load configuration: {0}")]
    LoadError(String),

    #[error("Failed to parse configuration: {0}")]
    ParseError(String),

    #[error("Configuration validation failed: {0}")]
    ValidationError(String),
}

impl From<ConfigCrateError> for ConfigError {
    fn from(err: ConfigCrateError) -> Self {
        ConfigError::LoadError(err.to_string())
    }
}

/// 配置文件搜索路径
const CONFIG_FILE_NAMES: &[&str] = &["config", "config.local"];

/// 加载应用配置
///
/// 按优先级从高到低合并配置：
/// 1. 环境变量（前缀 `ROVEL_`，层级分隔符 `__`）
/// 2. 配置文件（config.toml 或 config.local.toml）
/// 3. 默认值
///
/// # 环境变量示例
/// - `ROVEL_SERVER__HOST=127.0.0.1`
/// - `ROVEL_SERVER__PORT=8080`
/// - `ROVEL_TTS__URL=http://tts-server:8000`
/// - `ROVEL_DATABASE__PATH=/data/rovel.db`
///
/// # 返回
/// - `Ok(AppConfig)` - 成功加载的配置
/// - `Err(ConfigError)` - 加载失败
pub fn load_config() -> Result<AppConfig, ConfigError> {
    load_config_from_path(None)
}

/// 从指定路径加载配置
///
/// # 参数
/// - `config_path` - 可选的配置文件路径，如果为 None 则使用默认搜索路径
pub fn load_config_from_path(config_path: Option<&Path>) -> Result<AppConfig, ConfigError> {
    let mut builder = Config::builder();

    // 1. 首先设置默认值（最低优先级）
    builder = builder
        .set_default("server.host", "0.0.0.0")?
        .set_default("server.port", 5060)?
        .set_default("tts.url", "http://localhost:8000")?
        .set_default("tts.timeout_secs", 120)?
        .set_default("tts.max_retries", 0)?
        .set_default("database.path", "data/rovel.db")?
        .set_default("database.max_connections", 5)?
        .set_default("storage.audio_dir", "data/audio")?
        .set_default("storage.novels_dir", "data/novels")?
        .set_default("storage.voices_dir", "data/voices")?
        .set_default("storage.max_size_bytes", 0)?
        .set_default("storage.max_upload_size", 10 * 1024 * 1024)?
        .set_default("gc.enabled", true)?
        .set_default("gc.interval_secs", 3600)?
        .set_default("gc.session_expire_secs", 86400)?
        .set_default("gc.max_storage_bytes", 10_u64 * 1024 * 1024 * 1024)?
        .set_default("log.level", "info")?
        .set_default("log.json", false)?;

    // 2. 添加配置文件（如果存在）
    if let Some(path) = config_path {
        builder = builder.add_source(File::from(path).required(true));
    } else {
        // 搜索默认配置文件
        for name in CONFIG_FILE_NAMES {
            builder = builder.add_source(File::with_name(name).required(false));
        }
    }

    // 3. 添加环境变量（最高优先级）
    // 前缀: ROVEL_
    // 层级分隔符: __ (双下划线)
    // 例如: ROVEL_TTS__URL=http://tts-server:8000
    // 注意: 环境变量名会被转换为小写
    builder = builder.add_source(
        Environment::with_prefix("ROVEL")
            .prefix_separator("_")
            .separator("__")
            .try_parsing(true),
    );

    // 4. 构建配置
    let config = builder.build()?;

    // 5. 反序列化为 AppConfig
    let app_config: AppConfig = config.try_deserialize().map_err(|e| {
        ConfigError::ParseError(format!("Failed to deserialize config: {}", e))
    })?;

    // 6. 验证配置
    validate_config(&app_config)?;

    Ok(app_config)
}

/// 验证配置有效性
fn validate_config(config: &AppConfig) -> Result<(), ConfigError> {
    // 验证端口范围
    if config.server.port == 0 {
        return Err(ConfigError::ValidationError(
            "Server port cannot be 0".to_string(),
        ));
    }

    // 验证 TTS URL
    if config.tts.url.is_empty() {
        return Err(ConfigError::ValidationError(
            "TTS URL cannot be empty".to_string(),
        ));
    }

    // 验证数据库路径
    if config.database.path.is_empty() {
        return Err(ConfigError::ValidationError(
            "Database path cannot be empty".to_string(),
        ));
    }

    // 验证 GC 配置
    if config.gc.enabled && config.gc.interval_secs == 0 {
        return Err(ConfigError::ValidationError(
            "GC interval cannot be 0 when GC is enabled".to_string(),
        ));
    }

    Ok(())
}

/// 打印配置信息（用于启动时日志）
pub fn print_config(config: &AppConfig) {
    tracing::info!("=== Application Configuration ===");
    tracing::info!("Server: {}:{}", config.server.host, config.server.port);
    tracing::info!("Public Base URL: {}", config.server.public_base_url());
    tracing::info!("TTS URL: {}", config.tts.url);
    tracing::info!("TTS Timeout: {}s", config.tts.timeout_secs);
    tracing::info!("Database: {}", config.database.path);
    tracing::info!("Database Max Connections: {}", config.database.max_connections);
    tracing::info!("Audio Directory: {:?}", config.storage.audio_dir);
    tracing::info!("GC Enabled: {}", config.gc.enabled);
    if config.gc.enabled {
        tracing::info!("GC Interval: {}s", config.gc.interval_secs);
        tracing::info!("Session Expire: {}s", config.gc.session_expire_secs);
    }
    tracing::info!("Log Level: {}", config.log.level);
    tracing::info!("=================================");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_default_config() {
        // 使用临时配置文件测试默认值
        let config = AppConfig::default();
        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.server.port, 5060);
    }

    #[test]
    fn test_validation_passes_for_valid_config() {
        let config = AppConfig::default();
        assert!(validate_config(&config).is_ok());
    }

    #[test]
    fn test_validation_error_for_zero_port() {
        let mut config = AppConfig::default();
        config.server.port = 0;
        assert!(validate_config(&config).is_err());
    }

    #[test]
    fn test_validation_error_for_empty_tts_url() {
        let mut config = AppConfig::default();
        config.tts.url = String::new();
        assert!(validate_config(&config).is_err());
    }

    #[test]
    fn test_validation_error_for_empty_db_path() {
        let mut config = AppConfig::default();
        config.database.path = String::new();
        assert!(validate_config(&config).is_err());
    }
}
