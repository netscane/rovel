//! Inference Worker - Background TTS Task Processor

use std::sync::Arc;
use tokio::sync::mpsc;

use crate::application::ports::{
    generate_cache_key, AudioCachePort, CacheMetadata,
    SessionManagerPort,
    TaskManagerPort, TaskState,
    InferRequest, TtsEnginePort,
    VoiceRepositoryPort,
};
use crate::infrastructure::events::EventPublisher;

/// Worker 配置
#[derive(Debug, Clone)]
pub struct InferWorkerConfig {
    /// 最大并发推理数
    pub max_concurrent: usize,
    /// Rovel 服务的公开 Base URL（供 TTS 服务下载 voice reference）
    pub base_url: String,
}

impl Default for InferWorkerConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 2,
            base_url: "http://localhost:5060".to_string(),
        }
    }
}

/// 推理 Worker
///
/// 后台任务处理器，从队列消费任务并执行 TTS 推理
pub struct InferWorker {
    config: InferWorkerConfig,
    queue_receiver: mpsc::Receiver<String>,
    task_manager: Arc<dyn TaskManagerPort>,
    session_manager: Arc<dyn SessionManagerPort>,
    tts_engine: Arc<dyn TtsEnginePort>,
    audio_cache: Arc<dyn AudioCachePort>,
    voice_repo: Arc<dyn VoiceRepositoryPort>,
    event_publisher: Arc<EventPublisher>,
}

impl InferWorker {
    pub fn new(
        config: InferWorkerConfig,
        queue_receiver: mpsc::Receiver<String>,
        task_manager: Arc<dyn TaskManagerPort>,
        session_manager: Arc<dyn SessionManagerPort>,
        tts_engine: Arc<dyn TtsEnginePort>,
        audio_cache: Arc<dyn AudioCachePort>,
        voice_repo: Arc<dyn VoiceRepositoryPort>,
        event_publisher: Arc<EventPublisher>,
    ) -> Self {
        Self {
            config,
            queue_receiver,
            task_manager,
            session_manager,
            tts_engine,
            audio_cache,
            voice_repo,
            event_publisher,
        }
    }

    /// 启动 Worker
    pub async fn run(mut self) {
        tracing::info!(
            max_concurrent = self.config.max_concurrent,
            "InferWorker started"
        );

        // 使用 semaphore 控制并发
        let semaphore = Arc::new(tokio::sync::Semaphore::new(self.config.max_concurrent));

        while let Some(task_id) = self.queue_receiver.recv().await {
            let permit = semaphore.clone().acquire_owned().await;
            if permit.is_err() {
                tracing::error!("Failed to acquire semaphore permit");
                continue;
            }
            let permit = permit.unwrap();

            let task_manager = self.task_manager.clone();
            let session_manager = self.session_manager.clone();
            let tts_engine = self.tts_engine.clone();
            let audio_cache = self.audio_cache.clone();
            let voice_repo = self.voice_repo.clone();
            let event_publisher = self.event_publisher.clone();
            let base_url = self.config.base_url.clone();

            tokio::spawn(async move {
                let _permit = permit; // 持有 permit 直到任务完成

                Self::process_task(
                    &task_id,
                    task_manager,
                    session_manager,
                    tts_engine,
                    audio_cache,
                    voice_repo,
                    event_publisher,
                    &base_url,
                )
                .await;
            });
        }

        tracing::info!("InferWorker stopped");
    }

    /// 处理单个任务
    async fn process_task(
        task_id: &str,
        task_manager: Arc<dyn TaskManagerPort>,
        session_manager: Arc<dyn SessionManagerPort>,
        tts_engine: Arc<dyn TtsEnginePort>,
        audio_cache: Arc<dyn AudioCachePort>,
        voice_repo: Arc<dyn VoiceRepositoryPort>,
        event_publisher: Arc<EventPublisher>,
        base_url: &str,
    ) {
        // 获取任务信息
        let task = match task_manager.get_task(task_id) {
            Some(t) => t,
            None => {
                tracing::warn!(task_id = %task_id, "Task not found, skipping");
                return;
            }
        };

        // Check 1: 任务是否已取消
        if task_manager.is_cancelled(task_id) {
            tracing::debug!(task_id = %task_id, "Task cancelled, skipping");
            return;
        }

        // Check 2: 会话是否有效
        if !session_manager.is_valid(&task.session_id) {
            tracing::debug!(
                task_id = %task_id,
                session_id = %task.session_id,
                "Session invalid, skipping"
            );
            return;
        }

        // 检查缓存是否已存在
        let cache_key = generate_cache_key(&task.segment_content, &task.voice_id);
        if let Ok(Some(_)) = audio_cache.get(&cache_key).await {
            tracing::debug!(task_id = %task_id, "Cache hit, marking as ready");
            let _ = task_manager.set_state(task_id, TaskState::Ready);
            event_publisher.publish_task_ready(
                task_id,
                &task.session_id,
                task.segment_index,
            );
            return;
        }

        // 标记为推理中
        if let Err(e) = task_manager.set_state(task_id, TaskState::Inferring) {
            tracing::error!(task_id = %task_id, error = %e, "Failed to update task state");
            return;
        }
        event_publisher.publish_task_inferring(task_id, &task.session_id, task.segment_index);

        // 构建 voice reference 的下载 URL（TTS 服务通过此 URL 下载并缓存）
        let voice_ref = match voice_repo.find_by_id(task.voice_id).await {
            Ok(Some(_voice)) => {
                // 构建下载 URL: {base_url}/api/voice/audio/{voice_id}
                format!("{}/api/voice/audio/{}", base_url, task.voice_id)
            }
            Ok(None) => {
                tracing::error!(task_id = %task_id, voice_id = %task.voice_id, "Voice not found");
                let _ = task_manager.set_failed(task_id, "Voice not found".to_string());
                event_publisher.publish_task_failed(
                    task_id,
                    &task.session_id,
                    task.segment_index,
                    "Voice not found",
                );
                return;
            }
            Err(e) => {
                tracing::error!(task_id = %task_id, error = %e, "Failed to find voice");
                let _ = task_manager.set_failed(task_id, format!("Database error: {}", e));
                event_publisher.publish_task_failed(
                    task_id,
                    &task.session_id,
                    task.segment_index,
                    &format!("Database error: {}", e),
                );
                return;
            }
        };

        // 执行 TTS 推理
        let request = InferRequest {
            text: task.segment_content.clone(),
            voice_ref,
            voice_id: task.voice_id.to_string(),
        };

        let response = match tts_engine.infer(request).await {
            Ok(resp) => resp,
            Err(e) => {
                tracing::error!(task_id = %task_id, error = %e, "TTS inference failed");
                let _ = task_manager.set_failed(task_id, format!("TTS error: {}", e));
                event_publisher.publish_task_failed(
                    task_id,
                    &task.session_id,
                    task.segment_index,
                    &format!("TTS error: {}", e),
                );
                return;
            }
        };

        // Check 3: 推理后再次检查会话是否有效
        if !session_manager.is_valid(&task.session_id) {
            tracing::debug!(
                task_id = %task_id,
                session_id = %task.session_id,
                "Session invalid after TTS, dropping result"
            );
            return;
        }

        // 存储到缓存
        let metadata = CacheMetadata {
            novel_id: task.novel_id,
            segment_index: task.segment_index,
            voice_id: task.voice_id,
            content_hash: cache_key.clone(),
            duration_ms: response.duration_ms.unwrap_or(0),
            sample_rate: response.sample_rate,
        };

        if let Err(e) = audio_cache.put(&cache_key, response.audio_data, metadata).await {
            tracing::error!(task_id = %task_id, error = %e, "Failed to cache audio");
            let _ = task_manager.set_failed(task_id, format!("Cache error: {}", e));
            event_publisher.publish_task_failed(
                task_id,
                &task.session_id,
                task.segment_index,
                &format!("Cache error: {}", e),
            );
            return;
        }

        // 标记为完成
        let _ = task_manager.set_state(task_id, TaskState::Ready);
        event_publisher.publish_task_ready(task_id, &task.session_id, task.segment_index);

        tracing::info!(
            task_id = %task_id,
            session_id = %task.session_id,
            segment_index = task.segment_index,
            duration_ms = ?response.duration_ms,
            "Task completed"
        );
    }
}
