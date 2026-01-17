//! HTTP Server
//!
//! Axum HTTP 服务器启动和配置

use std::sync::Arc;

use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::middleware;
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use http::header::{AUTHORIZATION, CONTENT_TYPE};
use tracing::info;

use super::middleware::error_logging_middleware;
use super::routes::create_routes;
use super::state::AppState;

/// 服务器配置
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 5060,
        }
    }
}

impl ServerConfig {
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        Self {
            host: host.into(),
            port,
        }
    }

    pub fn addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

/// HTTP 服务器
pub struct HttpServer {
    config: ServerConfig,
    state: Arc<AppState>,
}

impl HttpServer {
    /// 创建新的 HTTP 服务器
    pub fn new(config: ServerConfig, state: AppState) -> Self {
        Self {
            config,
            state: Arc::new(state),
        }
    }

    /// 创建带默认配置的服务器
    pub fn with_default_config(state: AppState) -> Self {
        Self::new(ServerConfig::default(), state)
    }

    /// 构建 Router
    fn build_router(&self) -> Router {
        // CORS 配置 - 允许所有来源的跨域请求
        let cors = CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers([AUTHORIZATION, CONTENT_TYPE])
            .expose_headers(Any)
            .max_age(std::time::Duration::from_secs(3600));

        // 构建路由，设置请求体大小限制为 50MB（用于文件上传）
        create_routes()
            .layer(DefaultBodyLimit::max(50 * 1024 * 1024))
            .layer(middleware::from_fn(error_logging_middleware))
            .layer(TraceLayer::new_for_http())
            .layer(cors)
            .with_state(self.state.clone())
    }

    /// 启动服务器
    pub async fn run(self) -> Result<(), std::io::Error> {
        let router = self.build_router();
        let addr = self.config.addr();

        info!("Starting HTTP server on {}", addr);

        let listener = TcpListener::bind(&addr).await?;
        axum::serve(listener, router).await?;

        Ok(())
    }

    /// 启动服务器（带优雅关闭）
    pub async fn run_with_shutdown<F>(self, shutdown_signal: F) -> Result<(), std::io::Error>
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        let router = self.build_router();
        let addr = self.config.addr();

        info!("Starting HTTP server on {} (with graceful shutdown)", addr);

        let listener = TcpListener::bind(&addr).await?;
        axum::serve(listener, router)
            .with_graceful_shutdown(shutdown_signal)
            .await?;

        Ok(())
    }
}
