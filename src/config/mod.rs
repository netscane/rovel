//! Configuration Module
//!
//! 提供应用配置管理功能，支持多层级配置来源：
//! - 环境变量（最高优先级）
//! - 配置文件（TOML 格式）
//! - 默认值（最低优先级）

mod loader;
mod types;

pub use loader::{load_config, print_config, ConfigError};
pub use types::{
    AppConfig, DatabaseConfig, GcConfig, LogConfig, ServerConfig, StorageConfig, TtsConfig,
};
