//! SQLite Database - 数据库连接和迁移

use sqlx::{sqlite::SqlitePoolOptions, Pool, Sqlite};
use std::path::Path;

/// 数据库配置
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    /// 数据库文件路径
    pub database_url: String,
    /// 最大连接数
    pub max_connections: u32,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            database_url: "sqlite:./data/rovel.db?mode=rwc".to_string(),
            max_connections: 5,
        }
    }
}

impl DatabaseConfig {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            database_url: format!("sqlite:{}?mode=rwc", path.as_ref().display()),
            max_connections: 5,
        }
    }

    pub fn in_memory() -> Self {
        Self {
            database_url: "sqlite::memory:".to_string(),
            max_connections: 1,
        }
    }
}

/// 数据库连接池
pub type DbPool = Pool<Sqlite>;

/// 创建数据库连接池
pub async fn create_pool(config: &DatabaseConfig) -> Result<DbPool, sqlx::Error> {
    let pool = SqlitePoolOptions::new()
        .max_connections(config.max_connections)
        .connect(&config.database_url)
        .await?;

    // 启用 WAL 模式，允许并发读写
    sqlx::query("PRAGMA journal_mode=WAL")
        .execute(&pool)
        .await?;

    // 设置 busy_timeout=5000ms，遇到锁时等待而不是立即失败
    sqlx::query("PRAGMA busy_timeout=5000")
        .execute(&pool)
        .await?;

    // 设置同步模式为 NORMAL（平衡性能和安全性）
    sqlx::query("PRAGMA synchronous=NORMAL")
        .execute(&pool)
        .await?;

    tracing::info!("SQLite pool created with WAL mode and busy_timeout=5000ms");

    Ok(pool)
}

/// 运行数据库迁移
pub async fn run_migrations(pool: &DbPool) -> Result<(), sqlx::Error> {
    // 创建 novels 表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS novels (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            raw_text_path TEXT NOT NULL,
            total_segments INTEGER NOT NULL DEFAULT 0,
            status TEXT NOT NULL DEFAULT 'ready',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;

    // 创建 text_segments 表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS text_segments (
            id TEXT PRIMARY KEY,
            novel_id TEXT NOT NULL,
            segment_index INTEGER NOT NULL,
            content TEXT NOT NULL,
            char_count INTEGER NOT NULL,
            FOREIGN KEY (novel_id) REFERENCES novels(id) ON DELETE CASCADE,
            UNIQUE (novel_id, segment_index)
        )
        "#,
    )
    .execute(pool)
    .await?;

    // 创建 voices 表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS voices (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            reference_audio_path TEXT NOT NULL,
            description TEXT,
            created_at TEXT NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;

    // 创建 sessions 表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS sessions (
            id TEXT PRIMARY KEY,
            novel_id TEXT NOT NULL,
            voice_id TEXT NOT NULL,
            current_index INTEGER NOT NULL DEFAULT 0,
            state TEXT NOT NULL DEFAULT 'idle',
            window_before INTEGER NOT NULL DEFAULT 2,
            window_after INTEGER NOT NULL DEFAULT 3,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            last_accessed_at TEXT NOT NULL,
            FOREIGN KEY (novel_id) REFERENCES novels(id),
            FOREIGN KEY (voice_id) REFERENCES voices(id)
        )
        "#,
    )
    .execute(pool)
    .await?;

    // 创建 audio_segments 表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS audio_segments (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL,
            segment_index INTEGER NOT NULL,
            audio_path TEXT,
            duration_ms INTEGER,
            file_size INTEGER,
            state TEXT NOT NULL DEFAULT 'pending',
            error_message TEXT,
            created_at TEXT NOT NULL,
            last_accessed_at TEXT NOT NULL,
            FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE,
            UNIQUE (session_id, segment_index)
        )
        "#,
    )
    .execute(pool)
    .await?;

    // 创建索引
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_text_segments_novel_id 
        ON text_segments(novel_id)
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_audio_segments_session_id 
        ON audio_segments(session_id)
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_sessions_last_accessed 
        ON sessions(last_accessed_at)
        "#,
    )
    .execute(pool)
    .await?;

    // 索引: sessions.novel_id (用于级联删除)
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_sessions_novel_id 
        ON sessions(novel_id)
        "#,
    )
    .execute(pool)
    .await?;

    tracing::info!("Database migrations completed");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_in_memory_db() {
        let config = DatabaseConfig::in_memory();
        let pool = create_pool(&config).await.unwrap();
        run_migrations(&pool).await.unwrap();
    }
}
