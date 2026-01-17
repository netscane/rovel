//! Rovel - 有声小说 TTS 播放系统
//!
//! 架构设计: DDD + CQRS + Hexagonal Architecture
//!
//! 领域层 (domain/):
//! - Novel Context: 小说管理上下文
//! - Voice Context: 音色管理上下文
//!
//! 应用层 (application/):
//! - Ports: 端口定义（SessionManager, TaskManager, AudioCache, TtsEngine, Repositories）
//! - Commands: CQRS 命令处理器
//! - Queries: CQRS 查询处理器
//!
//! 基础设施层 (infrastructure/):
//! - HTTP: RESTful API + WebSocket
//! - Memory: SessionManager, TaskManager 内存实现
//! - Worker: InferWorker 后台任务处理
//! - Persistence: SQLite + Sled 存储
//! - Adapters: TTS Client, Text Segmenter
//! - Events: WebSocket 事件发布

pub mod application;
pub mod config;
pub mod domain;
pub mod infrastructure;

pub use config::{load_config, AppConfig};
