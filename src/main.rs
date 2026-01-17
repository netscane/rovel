//! Rovel - 有声小说 TTS 系统
//!
//! V2 架构 - 基于 ARCHITECTURE.md 设计:
//! - Domain: novel/, voice/ (Bounded Contexts)
//! - Application: commands, queries, ports
//! - Infrastructure: http, memory, worker, persistence, adapters, events

use std::sync::Arc;

use rovel::config::{load_config, print_config};
use rovel::infrastructure::adapters::{HttpTtsClient, HttpTtsClientConfig};
// use rovel::infrastructure::adapters::{FakeTtsClient, FakeTtsClientConfig};
use rovel::infrastructure::events::EventPublisher;
use rovel::infrastructure::http::{AppState, HttpServer, ServerConfig};
use rovel::infrastructure::memory::{InMemorySessionManager, InMemoryTaskManager};
use rovel::infrastructure::persistence::sled::{SledAudioCache, SledCacheConfig};
use rovel::infrastructure::persistence::sqlite::{
    create_pool, run_migrations, DatabaseConfig,
    SqliteNovelRepository, SqliteVoiceRepository,
};
use rovel::infrastructure::worker::{InferWorker, InferWorkerConfig};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 加载配置（优先级：环境变量 > 配置文件 > 默认值）
    let config = load_config().map_err(|e| anyhow::anyhow!("Failed to load config: {}", e))?;

    // 初始化日志
    let log_filter = format!(
        "{},rovel={},tower_http=debug",
        config.log.level, config.log.level
    );
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&log_filter)),
        )
        .init();

    tracing::info!("Rovel - 有声小说 TTS 系统 (V2 架构)");
    print_config(&config);

    // 确保数据目录存在
    tokio::fs::create_dir_all(&config.storage.audio_dir).await?;
    if let Some(parent) = std::path::Path::new(&config.database.path).parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    // 初始化数据库
    let db_config = DatabaseConfig {
        database_url: config.database.database_url(),
        max_connections: config.database.max_connections,
    };
    let pool = create_pool(&db_config).await?;
    run_migrations(&pool).await?;

    // 创建 Repository 适配器
    let novel_repo = Arc::new(SqliteNovelRepository::new(pool.clone()));
    let voice_repo = Arc::new(SqliteVoiceRepository::new(pool.clone()));

    // 创建 HTTP TTS 引擎
    let tts_config = HttpTtsClientConfig {
        base_url: config.tts.url.clone(),
        timeout_secs: config.tts.timeout_secs,
        max_retries: config.tts.max_retries,
    };
    let tts_engine = Arc::new(HttpTtsClient::new(tts_config)?);

    // // 创建 Fake TTS 引擎（测试用，始终返回固定音频）
    // let tts_config = FakeTtsClientConfig {
    //     audio_file_path: std::path::PathBuf::from("/home/github/rovel/Speaker_1.wav"),
    //     duration_ms: 5000,
    //     sample_rate: 22050,
    // };
    // let tts_engine = Arc::new(FakeTtsClient::new(tts_config)?);;

    // 创建 Sled 音频缓存
    let cache_config = SledCacheConfig {
        db_path: format!("{}/cache.sled", config.storage.audio_dir.display()),
        max_size_bytes: 10 * 1024 * 1024 * 1024, // 10GB
    };
    let audio_cache = Arc::new(SledAudioCache::new(&cache_config)?);

    // 创建事件发布器
    let event_publisher = Arc::new(EventPublisher::new());

    // 创建任务队列
    let (task_tx, task_rx) = mpsc::channel(1000);

    // 创建内存 Session 和 Task 管理器
    let session_manager = Arc::new(InMemorySessionManager::new());
    let task_manager = Arc::new(InMemoryTaskManager::new(task_tx));

    // 创建 InferWorker
    let worker_config = InferWorkerConfig {
        max_concurrent: 2,
        base_url: config.server.public_base_url(),
    };
    let worker = InferWorker::new(
        worker_config,
        task_rx,
        task_manager.clone(),
        session_manager.clone(),
        tts_engine.clone(),
        audio_cache.clone(),
        voice_repo.clone(),
        event_publisher.clone(),
    );

    // 启动 Worker
    tokio::spawn(worker.run());

    // 创建 HTTP 服务器
    let server_config = ServerConfig::new(&config.server.host, config.server.port);
    let state = AppState::new(
        session_manager,
        task_manager,
        novel_repo,
        voice_repo,
        audio_cache,
        tts_engine,
        event_publisher,
    );

    let server = HttpServer::new(server_config, state);

    tracing::info!("Starting HTTP server...");

    // 启动服务器（带优雅关闭）
    server
        .run_with_shutdown(async {
            tokio::signal::ctrl_c()
                .await
                .expect("Failed to listen for ctrl-c");
            tracing::info!("Received shutdown signal");
        })
        .await?;

    tracing::info!("Server shutdown complete");

    Ok(())
}
