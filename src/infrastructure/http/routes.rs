//! HTTP Routes
//!
//! API 路由定义 - 基于 ARCHITECTURE.md V2 设计
//!
//! API Endpoints:
//! - /api/novel/upload      POST  上传小说（异步处理，通过 WS 通知完成）
//! - /api/novel/delete      POST  删除小说
//! - /api/novel/get         POST  获取小说详情
//! - /api/novel/list        GET   列出所有小说
//! - /api/novel/segments    POST  获取小说片段
//! - /api/voice/upload      POST  上传音色
//! - /api/voice/delete      POST  删除音色
//! - /api/voice/get         POST  获取音色详情
//! - /api/voice/list        GET   列出所有音色
//! - /api/session/play      POST  开始播放（创建会话）
//! - /api/session/seek      POST  跳转位置
//! - /api/session/change_voice POST 切换音色
//! - /api/session/close     POST  关闭会话
//! - /api/infer/submit      POST  提交推理任务
//! - /api/infer/status      POST  查询任务状态
//! - /api/audio             POST  获取音频
//! - /ws/session/{id}       WS    Session WebSocket（task 状态事件）
//! - /ws/events             WS    全局 WebSocket（novel 事件）

use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;

use super::handlers;
use super::state::AppState;

/// 创建所有路由
pub fn create_routes() -> Router<Arc<AppState>> {
    Router::new()
        .nest("/api", api_routes())
        .route("/ws/session/:session_id", get(handlers::websocket_handler))
        .route("/ws/events", get(handlers::global_websocket_handler))
}

/// API 路由
fn api_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/ping", get(handlers::ping))
        .nest("/novel", novel_routes())
        .nest("/voice", voice_routes())
        .nest("/session", session_routes())
        .nest("/infer", infer_routes())
        .route("/audio", post(handlers::get_audio))
}

/// Novel 路由
fn novel_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/upload", post(handlers::upload_novel))
        .route("/delete", post(handlers::delete_novel))
        .route("/get", post(handlers::get_novel))
        .route("/list", get(handlers::list_novels))
        .route("/segments", post(handlers::get_novel_segments))
}

/// Voice 路由
fn voice_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/upload", post(handlers::upload_voice))
        .route("/delete", post(handlers::delete_voice))
        .route("/get", post(handlers::get_voice))
        .route("/list", get(handlers::list_voices))
        .route("/audio/:voice_id", get(handlers::download_voice_audio))
}

/// Session 路由
fn session_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/play", post(handlers::play))
        .route("/seek", post(handlers::seek))
        .route("/change_voice", post(handlers::change_voice))
        .route("/close", post(handlers::close_session))
}

/// Infer 路由
fn infer_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/submit", post(handlers::submit_infer))
        .route("/status", post(handlers::query_task_status))
}
