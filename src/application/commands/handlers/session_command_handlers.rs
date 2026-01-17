//! Session Command Handlers - V2 架构

use std::sync::Arc;

use crate::application::commands::session_commands::*;
use crate::application::error::ApplicationError;
use crate::application::ports::{
    NovelRepositoryPort, Session, SessionManagerPort, TaskManagerPort, VoiceRepositoryPort,
};
use crate::infrastructure::events::EventPublisher;

/// Play Handler - 创建或复用会话
pub struct PlayHandler {
    session_manager: Arc<dyn SessionManagerPort>,
    task_manager: Arc<dyn TaskManagerPort>,
    novel_repo: Arc<dyn NovelRepositoryPort>,
    voice_repo: Arc<dyn VoiceRepositoryPort>,
}

impl PlayHandler {
    pub fn new(
        session_manager: Arc<dyn SessionManagerPort>,
        task_manager: Arc<dyn TaskManagerPort>,
        novel_repo: Arc<dyn NovelRepositoryPort>,
        voice_repo: Arc<dyn VoiceRepositoryPort>,
    ) -> Self {
        Self {
            session_manager,
            task_manager,
            novel_repo,
            voice_repo,
        }
    }

    pub async fn handle(&self, cmd: PlayCommand) -> Result<PlayResponse, ApplicationError> {
        // 验证 novel 存在
        let novel = self
            .novel_repo
            .find_by_id(cmd.novel_id)
            .await?
            .ok_or_else(|| ApplicationError::not_found("Novel", cmd.novel_id))?;

        // 验证 voice 存在
        self.voice_repo
            .find_by_id(cmd.voice_id)
            .await?
            .ok_or_else(|| ApplicationError::not_found("Voice", cmd.voice_id))?;

        // 验证 start_index 有效
        if cmd.start_index as usize >= novel.total_segments {
            return Err(ApplicationError::validation(format!(
                "Invalid start_index: {} (total segments: {})",
                cmd.start_index, novel.total_segments
            )));
        }

        // 创建新会话
        let session = Session::new(cmd.novel_id, cmd.voice_id, cmd.start_index);
        let session_id = self
            .session_manager
            .create(session)
            .map_err(|e| ApplicationError::internal(e.to_string()))?;

        tracing::info!(
            session_id = %session_id,
            novel_id = %cmd.novel_id,
            voice_id = %cmd.voice_id,
            start_index = cmd.start_index,
            "Play session created"
        );

        Ok(PlayResponse {
            session_id,
            novel_id: cmd.novel_id,
            voice_id: cmd.voice_id,
            current_index: cmd.start_index,
        })
    }
}

/// Seek Handler - 跳转位置并取消 pending 任务
pub struct SeekHandler {
    session_manager: Arc<dyn SessionManagerPort>,
    task_manager: Arc<dyn TaskManagerPort>,
}

impl SeekHandler {
    pub fn new(
        session_manager: Arc<dyn SessionManagerPort>,
        task_manager: Arc<dyn TaskManagerPort>,
    ) -> Self {
        Self {
            session_manager,
            task_manager,
        }
    }

    pub async fn handle(&self, cmd: SeekCommand) -> Result<SeekResponse, ApplicationError> {
        // 验证会话存在
        let _session = self
            .session_manager
            .get(&cmd.session_id)
            .map_err(|_| ApplicationError::not_found_str("Session", &cmd.session_id))?;

        // 取消所有 pending 任务
        let cancelled_count = self.task_manager.cancel_pending(&cmd.session_id);

        // 更新当前索引
        self.session_manager
            .update_index(&cmd.session_id, cmd.segment_index)
            .map_err(|e| ApplicationError::internal(e.to_string()))?;

        tracing::info!(
            session_id = %cmd.session_id,
            segment_index = cmd.segment_index,
            cancelled_count = cancelled_count,
            "Session seeked"
        );

        Ok(SeekResponse {
            session_id: cmd.session_id,
            current_index: cmd.segment_index,
            cancelled_count,
        })
    }
}

/// ChangeVoice Handler - 切换音色并取消所有任务
pub struct ChangeVoiceHandler {
    session_manager: Arc<dyn SessionManagerPort>,
    task_manager: Arc<dyn TaskManagerPort>,
    voice_repo: Arc<dyn VoiceRepositoryPort>,
}

impl ChangeVoiceHandler {
    pub fn new(
        session_manager: Arc<dyn SessionManagerPort>,
        task_manager: Arc<dyn TaskManagerPort>,
        voice_repo: Arc<dyn VoiceRepositoryPort>,
    ) -> Self {
        Self {
            session_manager,
            task_manager,
            voice_repo,
        }
    }

    pub async fn handle(&self, cmd: ChangeVoiceCommand) -> Result<ChangeVoiceResponse, ApplicationError> {
        // 验证会话存在
        self.session_manager
            .get(&cmd.session_id)
            .map_err(|_| ApplicationError::not_found_str("Session", &cmd.session_id))?;

        // 验证 voice 存在
        self.voice_repo
            .find_by_id(cmd.voice_id)
            .await?
            .ok_or_else(|| ApplicationError::not_found("Voice", cmd.voice_id))?;

        // 取消所有 pending 任务
        let cancelled_count = self.task_manager.cancel_pending(&cmd.session_id);

        // 更新音色
        self.session_manager
            .update_voice(&cmd.session_id, cmd.voice_id)
            .map_err(|e| ApplicationError::internal(e.to_string()))?;

        tracing::info!(
            session_id = %cmd.session_id,
            voice_id = %cmd.voice_id,
            cancelled_count = cancelled_count,
            "Session voice changed"
        );

        Ok(ChangeVoiceResponse {
            session_id: cmd.session_id,
            voice_id: cmd.voice_id,
            cancelled_count,
        })
    }
}

/// CloseSession Handler - 关闭会话
pub struct CloseSessionHandler {
    session_manager: Arc<dyn SessionManagerPort>,
    task_manager: Arc<dyn TaskManagerPort>,
    event_publisher: Arc<EventPublisher>,
}

impl CloseSessionHandler {
    pub fn new(
        session_manager: Arc<dyn SessionManagerPort>,
        task_manager: Arc<dyn TaskManagerPort>,
        event_publisher: Arc<EventPublisher>,
    ) -> Self {
        Self {
            session_manager,
            task_manager,
            event_publisher,
        }
    }

    pub async fn handle(&self, cmd: CloseSessionCommand) -> Result<CloseSessionResponse, ApplicationError> {
        // 取消所有 pending 任务
        let cancelled = self.task_manager.cancel_pending(&cmd.session_id);

        // 清理任务
        self.task_manager.cleanup_session(&cmd.session_id);

        // 发布会话关闭事件
        self.event_publisher.publish_session_closed(&cmd.session_id, "client_close");

        // 关闭会话
        self.session_manager
            .close(&cmd.session_id)
            .map_err(|_| ApplicationError::not_found_str("Session", &cmd.session_id))?;

        // 取消注册事件通道
        self.event_publisher.unregister_session(&cmd.session_id);

        tracing::info!(
            session_id = %cmd.session_id,
            cancelled_tasks = cancelled,
            "Session closed"
        );

        Ok(CloseSessionResponse {
            session_id: cmd.session_id,
        })
    }
}
