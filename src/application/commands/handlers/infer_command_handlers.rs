//! Inference Command Handlers - V2 架构

use std::sync::Arc;

use crate::application::commands::infer_commands::*;
use crate::application::error::ApplicationError;
use crate::application::ports::{
    generate_cache_key, AudioCachePort, InferenceTask, NovelRepositoryPort, SessionManagerPort,
    TaskManagerPort, TaskState,
};

/// SubmitInfer Handler - 提交推理任务
pub struct SubmitInferHandler {
    session_manager: Arc<dyn SessionManagerPort>,
    task_manager: Arc<dyn TaskManagerPort>,
    novel_repo: Arc<dyn NovelRepositoryPort>,
    audio_cache: Arc<dyn AudioCachePort>,
}

impl SubmitInferHandler {
    pub fn new(
        session_manager: Arc<dyn SessionManagerPort>,
        task_manager: Arc<dyn TaskManagerPort>,
        novel_repo: Arc<dyn NovelRepositoryPort>,
        audio_cache: Arc<dyn AudioCachePort>,
    ) -> Self {
        Self {
            session_manager,
            task_manager,
            novel_repo,
            audio_cache,
        }
    }

    pub async fn handle(&self, cmd: SubmitInferCommand) -> Result<SubmitInferResponse, ApplicationError> {
        // 获取会话信息
        let session = self
            .session_manager
            .get(&cmd.session_id)
            .map_err(|_| ApplicationError::not_found_str("Session", &cmd.session_id))?;

        // 只获取需要的段落（而不是所有段落）
        let segments = self
            .novel_repo
            .find_segments_by_indices(session.novel_id, &cmd.segment_indices)
            .await?;

        tracing::info!(
            session_id = %cmd.session_id,
            requested_indices = ?cmd.segment_indices,
            found_segments = segments.len(),
            "Fetched segments for inference"
        );

        let mut tasks_to_submit = Vec::new();
        let mut response_tasks = Vec::new();

        for segment_index in cmd.segment_indices.iter().copied() {
            // 验证索引有效
            let segment = segments
                .iter()
                .find(|s| s.index == segment_index as usize)
                .ok_or_else(|| {
                    ApplicationError::validation(format!("Invalid segment index: {}", segment_index))
                })?;

            // 检查缓存是否已存在
            let cache_key = generate_cache_key(&segment.content, &session.voice_id);
            let cache_exists = self.audio_cache.exists(&cache_key).await;
            tracing::info!(
                segment_index = segment_index,
                cache_key = %cache_key,
                cache_exists = ?cache_exists,
                "Checking cache"
            );
            if let Ok(true) = cache_exists {
                // 缓存命中，直接返回 ready 状态
                tracing::info!(segment_index = segment_index, "Cache hit");
                response_tasks.push(TaskInfo {
                    task_id: format!("cached-{}-{}", session.novel_id, segment_index),
                    segment_index,
                    state: TaskState::Ready,
                });
                continue;
            }

            // 创建推理任务
            let task = InferenceTask::new(
                cmd.session_id.clone(),
                session.novel_id,
                session.voice_id,
                segment_index,
                segment.content.clone(),
            );

            tracing::debug!(
                task_id = %task.task_id,
                segment_index = segment_index,
                "Creating inference task"
            );

            response_tasks.push(TaskInfo {
                task_id: task.task_id.clone(),
                segment_index,
                state: TaskState::Pending,
            });

            tasks_to_submit.push(task);
        }

        // 批量提交任务
        if !tasks_to_submit.is_empty() {
            tracing::info!(
                count = tasks_to_submit.len(),
                "Submitting tasks to queue"
            );
            self.task_manager
                .submit(tasks_to_submit)
                .map_err(|e| ApplicationError::internal(e.to_string()))?;
        }

        tracing::debug!(
            session_id = %cmd.session_id,
            submitted = response_tasks.len(),
            "Inference tasks submitted"
        );

        Ok(SubmitInferResponse {
            tasks: response_tasks,
        })
    }
}

/// QueryTaskStatus Handler - 查询任务状态
pub struct QueryTaskStatusHandler {
    task_manager: Arc<dyn TaskManagerPort>,
}

impl QueryTaskStatusHandler {
    pub fn new(task_manager: Arc<dyn TaskManagerPort>) -> Self {
        Self { task_manager }
    }

    pub fn handle(&self, cmd: QueryTaskStatusCommand) -> QueryTaskStatusResponse {
        let tasks = cmd
            .task_ids
            .iter()
            .filter_map(|task_id| {
                self.task_manager.get_task(task_id).map(|task| TaskStatusInfo {
                    task_id: task.task_id,
                    segment_index: task.segment_index,
                    state: task.state,
                    error: task.error_message,
                })
            })
            .collect();

        QueryTaskStatusResponse { tasks }
    }
}
