//! WebSocket Handler - V2 架构

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;

use crate::infrastructure::http::state::AppState;
use crate::infrastructure::events::WsEvent;

/// Session WebSocket 连接处理（用于 task 状态通知）
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    Path(session_id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_session_socket(socket, session_id, state))
}

/// 全局 WebSocket 连接处理（用于 novel 事件通知）
pub async fn global_websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_global_socket(socket, state))
}

async fn handle_session_socket(socket: WebSocket, session_id: String, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();

    // 验证会话存在
    if !state.session_manager.is_valid(&session_id) {
        tracing::warn!(session_id = %session_id, "WebSocket connection rejected: invalid session");
        let _ = sender.close().await;
        return;
    }

    // 注册事件接收器
    let mut event_rx = state.event_publisher.register_session(&session_id);

    tracing::info!(session_id = %session_id, "WebSocket connected");

    // Clone session_id for different tasks
    let session_id_for_forward = session_id.clone();
    let session_id_for_receive = session_id.clone();
    let session_id_for_cleanup = session_id.clone();

    // 事件转发任务
    let forward_task = tokio::spawn(async move {
        while let Ok(event) = event_rx.recv().await {
            let msg = match serde_json::to_string(&event) {
                Ok(json) => Message::Text(json),
                Err(e) => {
                    tracing::error!(error = %e, "Failed to serialize event");
                    continue;
                }
            };

            if let Err(e) = sender.send(msg).await {
                tracing::debug!(
                    session_id = %session_id_for_forward,
                    error = %e,
                    "Failed to send WebSocket message"
                );
                break;
            }
        }
    });

    // 接收客户端消息（心跳）
    let session_manager = state.session_manager.clone();
    let receive_task = tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            match msg {
                Ok(Message::Ping(_)) => {
                    // 自动响应 pong（由 axum 处理）
                    session_manager.touch(&session_id_for_receive);
                }
                Ok(Message::Close(_)) => {
                    tracing::info!(session_id = %session_id_for_receive, "WebSocket closed by client");
                    break;
                }
                Err(e) => {
                    tracing::debug!(session_id = %session_id_for_receive, error = %e, "WebSocket error");
                    break;
                }
                _ => {
                    // 其他消息类型 - touch session
                    session_manager.touch(&session_id_for_receive);
                }
            }
        }
    });

    // 等待任一任务完成
    tokio::select! {
        _ = forward_task => {}
        _ = receive_task => {}
    }

    // 清理
    state.event_publisher.unregister_session(&session_id_for_cleanup);
    tracing::info!(session_id = %session_id_for_cleanup, "WebSocket disconnected");
}

/// 处理全局 WebSocket（用于接收 NovelReady/NovelFailed 事件）
async fn handle_global_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();

    // 订阅全局事件
    let mut event_rx = state.event_publisher.subscribe_global();

    tracing::info!("Global WebSocket connected");

    // 事件转发任务
    let forward_task = tokio::spawn(async move {
        while let Ok(event) = event_rx.recv().await {
            // 转发全局事件（Novel 和 Voice 相关）
            match &event {
                WsEvent::NovelReady { .. }
                | WsEvent::NovelFailed { .. }
                | WsEvent::NovelDeleting { .. }
                | WsEvent::NovelDeleted { .. }
                | WsEvent::NovelDeleteFailed { .. }
                | WsEvent::VoiceDeleted { .. } => {
                    let msg = match serde_json::to_string(&event) {
                        Ok(json) => Message::Text(json),
                        Err(e) => {
                            tracing::error!(error = %e, "Failed to serialize event");
                            continue;
                        }
                    };

                    if let Err(e) = sender.send(msg).await {
                        tracing::debug!(error = %e, "Failed to send global WebSocket message");
                        break;
                    }
                }
                _ => {}
            }
        }
    });

    // 接收客户端消息（心跳）
    let receive_task = tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            match msg {
                Ok(Message::Ping(_)) => {
                    // 自动响应 pong
                }
                Ok(Message::Close(_)) => {
                    tracing::info!("Global WebSocket closed by client");
                    break;
                }
                Err(e) => {
                    tracing::debug!(error = %e, "Global WebSocket error");
                    break;
                }
                _ => {}
            }
        }
    });

    // 等待任一任务完成
    tokio::select! {
        _ = forward_task => {}
        _ = receive_task => {}
    }

    tracing::info!("Global WebSocket disconnected");
}
