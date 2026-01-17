//! Event Publisher Implementation
//!
//! WebSocket 事件推送实现

use crate::application::ports::TaskState;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;
use uuid::Uuid;

/// WebSocket 事件类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", content = "data")]
pub enum WsEvent {
    /// 任务状态变更
    TaskStateChanged {
        session_id: String,
        task_id: String,
        segment_index: u32,
        state: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        duration_ms: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },
    /// 会话关闭
    SessionClosed {
        session_id: String,
        reason: String,
    },
    /// Novel 处理完成
    NovelReady {
        novel_id: Uuid,
        title: String,
        total_segments: usize,
    },
    /// Novel 处理失败
    NovelFailed {
        novel_id: Uuid,
        error: String,
    },
    /// Novel 删除中
    NovelDeleting {
        novel_id: Uuid,
    },
    /// Novel 删除完成
    NovelDeleted {
        novel_id: Uuid,
    },
    /// Novel 删除失败
    NovelDeleteFailed {
        novel_id: Uuid,
        error: String,
    },
    /// Voice 删除完成
    VoiceDeleted {
        voice_id: Uuid,
    },
}

/// 事件发布器
pub struct EventPublisher {
    /// session_id -> broadcast sender (for session-specific events)
    session_channels: DashMap<String, broadcast::Sender<WsEvent>>,
    /// Global broadcast channel for novel events (NovelReady/NovelFailed)
    global_channel: broadcast::Sender<WsEvent>,
}

impl EventPublisher {
    pub fn new() -> Self {
        let (global_tx, _) = broadcast::channel(100);
        Self {
            session_channels: DashMap::new(),
            global_channel: global_tx,
        }
    }

    pub fn arc(self) -> Arc<Self> {
        Arc::new(self)
    }

    /// 订阅全局事件（NovelReady/NovelFailed）
    pub fn subscribe_global(&self) -> broadcast::Receiver<WsEvent> {
        self.global_channel.subscribe()
    }

    /// 注册会话的事件通道
    pub fn register_session(&self, session_id: &str) -> broadcast::Receiver<WsEvent> {
        if let Some(sender) = self.session_channels.get(session_id) {
            return sender.subscribe();
        }

        let (tx, rx) = broadcast::channel(100);
        self.session_channels.insert(session_id.to_string(), tx);
        rx
    }

    /// 取消注册会话
    pub fn unregister_session(&self, session_id: &str) {
        self.session_channels.remove(session_id);
    }

    /// 获取会话的事件接收器
    pub fn subscribe(&self, session_id: &str) -> Option<broadcast::Receiver<WsEvent>> {
        self.session_channels.get(session_id).map(|s| s.subscribe())
    }

    /// 发布任务开始推理事件
    pub fn publish_task_inferring(&self, task_id: &str, session_id: &str, segment_index: u32) {
        self.publish_to_session(
            session_id,
            WsEvent::TaskStateChanged {
                session_id: session_id.to_string(),
                task_id: task_id.to_string(),
                segment_index,
                state: TaskState::Inferring.as_str().to_string(),
                duration_ms: None,
                error: None,
            },
        );
    }

    /// 发布任务完成事件
    pub fn publish_task_ready(&self, task_id: &str, session_id: &str, segment_index: u32) {
        self.publish_to_session(
            session_id,
            WsEvent::TaskStateChanged {
                session_id: session_id.to_string(),
                task_id: task_id.to_string(),
                segment_index,
                state: TaskState::Ready.as_str().to_string(),
                duration_ms: None,
                error: None,
            },
        );
    }

    /// 发布任务完成事件（带时长）
    pub fn publish_task_ready_with_duration(
        &self,
        task_id: &str,
        session_id: &str,
        segment_index: u32,
        duration_ms: u64,
    ) {
        self.publish_to_session(
            session_id,
            WsEvent::TaskStateChanged {
                session_id: session_id.to_string(),
                task_id: task_id.to_string(),
                segment_index,
                state: TaskState::Ready.as_str().to_string(),
                duration_ms: Some(duration_ms),
                error: None,
            },
        );
    }

    /// 发布任务失败事件
    pub fn publish_task_failed(
        &self,
        task_id: &str,
        session_id: &str,
        segment_index: u32,
        error: &str,
    ) {
        self.publish_to_session(
            session_id,
            WsEvent::TaskStateChanged {
                session_id: session_id.to_string(),
                task_id: task_id.to_string(),
                segment_index,
                state: TaskState::Failed.as_str().to_string(),
                duration_ms: None,
                error: Some(error.to_string()),
            },
        );
    }

    /// 发布会话关闭事件
    pub fn publish_session_closed(&self, session_id: &str, reason: &str) {
        self.publish_to_session(
            session_id,
            WsEvent::SessionClosed {
                session_id: session_id.to_string(),
                reason: reason.to_string(),
            },
        );
    }

    /// 发布 Novel 处理完成事件（全局广播）
    pub fn publish_novel_ready(&self, novel_id: Uuid, title: &str, total_segments: usize) {
        let event = WsEvent::NovelReady {
            novel_id,
            title: title.to_string(),
            total_segments,
        };
        if let Err(e) = self.global_channel.send(event) {
            tracing::debug!(
                novel_id = %novel_id,
                error = %e,
                "Failed to publish NovelReady event (no receivers)"
            );
        }
    }

    /// 发布 Novel 处理失败事件（全局广播）
    pub fn publish_novel_failed(&self, novel_id: Uuid, error: &str) {
        let event = WsEvent::NovelFailed {
            novel_id,
            error: error.to_string(),
        };
        if let Err(e) = self.global_channel.send(event) {
            tracing::debug!(
                novel_id = %novel_id,
                error = %e,
                "Failed to publish NovelFailed event (no receivers)"
            );
        }
    }

    /// 发布 Novel 删除中事件（全局广播）
    pub fn publish_novel_deleting(&self, novel_id: Uuid) {
        let event = WsEvent::NovelDeleting { novel_id };
        if let Err(e) = self.global_channel.send(event) {
            tracing::debug!(
                novel_id = %novel_id,
                error = %e,
                "Failed to publish NovelDeleting event (no receivers)"
            );
        }
    }

    /// 发布 Novel 删除完成事件（全局广播）
    pub fn publish_novel_deleted(&self, novel_id: Uuid) {
        let event = WsEvent::NovelDeleted { novel_id };
        if let Err(e) = self.global_channel.send(event) {
            tracing::debug!(
                novel_id = %novel_id,
                error = %e,
                "Failed to publish NovelDeleted event (no receivers)"
            );
        }
    }

    /// 发布 Novel 删除失败事件（全局广播）
    pub fn publish_novel_delete_failed(&self, novel_id: Uuid, error: &str) {
        let event = WsEvent::NovelDeleteFailed {
            novel_id,
            error: error.to_string(),
        };
        if let Err(e) = self.global_channel.send(event) {
            tracing::debug!(
                novel_id = %novel_id,
                error = %e,
                "Failed to publish NovelDeleteFailed event (no receivers)"
            );
        }
    }

    /// 发布 Voice 删除完成事件（全局广播）
    pub fn publish_voice_deleted(&self, voice_id: Uuid) {
        let event = WsEvent::VoiceDeleted { voice_id };
        if let Err(e) = self.global_channel.send(event) {
            tracing::debug!(
                voice_id = %voice_id,
                error = %e,
                "Failed to publish VoiceDeleted event (no receivers)"
            );
        }
    }

    /// 发布事件到指定会话
    fn publish_to_session(&self, session_id: &str, event: WsEvent) {
        if let Some(sender) = self.session_channels.get(session_id) {
            if let Err(e) = sender.send(event) {
                tracing::debug!(
                    session_id = %session_id,
                    error = %e,
                    "Failed to publish event (no receivers)"
                );
            }
        }
    }
}

impl Default for EventPublisher {
    fn default() -> Self {
        Self::new()
    }
}
