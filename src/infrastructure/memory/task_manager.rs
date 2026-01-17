//! In-Memory Task Manager Implementation

use chrono::Utc;
use dashmap::DashMap;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::application::ports::{InferenceTask, TaskError, TaskManagerPort, TaskState};

/// 内存任务管理器
pub struct InMemoryTaskManager {
    /// task_id -> InferenceTask
    tasks: DashMap<String, InferenceTask>,
    /// session_id -> Set<task_id>
    session_tasks: DashMap<String, HashSet<String>>,
    /// 任务队列发送端
    queue_sender: mpsc::Sender<String>,
}

impl InMemoryTaskManager {
    pub fn new(queue_sender: mpsc::Sender<String>) -> Self {
        Self {
            tasks: DashMap::new(),
            session_tasks: DashMap::new(),
            queue_sender,
        }
    }

    pub fn arc(self) -> Arc<Self> {
        Arc::new(self)
    }
}

impl TaskManagerPort for InMemoryTaskManager {
    fn submit(&self, tasks: Vec<InferenceTask>) -> Result<Vec<String>, TaskError> {
        let mut task_ids = Vec::with_capacity(tasks.len());

        for task in tasks {
            let task_id = task.task_id.clone();
            let session_id = task.session_id.clone();

            // 存储任务
            self.tasks.insert(task_id.clone(), task);

            // 关联到会话
            self.session_tasks
                .entry(session_id.clone())
                .or_insert_with(HashSet::new)
                .insert(task_id.clone());

            // 发送到队列
            if let Err(e) = self.queue_sender.try_send(task_id.clone()) {
                tracing::warn!(task_id = %task_id, error = %e, "Failed to enqueue task");
            }

            task_ids.push(task_id);
        }

        tracing::debug!(count = task_ids.len(), "Tasks submitted");
        Ok(task_ids)
    }

    fn cancel_pending(&self, session_id: &str) -> usize {
        let mut cancelled_count = 0;

        if let Some(task_ids) = self.session_tasks.get(session_id) {
            for task_id in task_ids.iter() {
                if let Some(mut task) = self.tasks.get_mut(task_id) {
                    if task.state == TaskState::Pending {
                        task.state = TaskState::Cancelled;
                        task.completed_at = Some(Utc::now());
                        cancelled_count += 1;
                    }
                }
            }
        }

        tracing::debug!(
            session_id = %session_id,
            cancelled_count = cancelled_count,
            "Pending tasks cancelled"
        );
        cancelled_count
    }

    fn is_cancelled(&self, task_id: &str) -> bool {
        self.tasks
            .get(task_id)
            .map(|t| t.state == TaskState::Cancelled)
            .unwrap_or(true) // 不存在的任务视为已取消
    }

    fn get_state(&self, task_id: &str) -> Option<TaskState> {
        self.tasks.get(task_id).map(|t| t.state)
    }

    fn set_state(&self, task_id: &str, state: TaskState) -> Result<(), TaskError> {
        let mut task = self
            .tasks
            .get_mut(task_id)
            .ok_or_else(|| TaskError::NotFound(task_id.to_string()))?;

        let old_state = task.state;
        task.state = state;

        if matches!(state, TaskState::Ready | TaskState::Failed | TaskState::Cancelled) {
            task.completed_at = Some(Utc::now());
        }

        tracing::debug!(
            task_id = %task_id,
            old_state = ?old_state,
            new_state = ?state,
            "Task state changed"
        );
        Ok(())
    }

    fn set_failed(&self, task_id: &str, error: String) -> Result<(), TaskError> {
        let mut task = self
            .tasks
            .get_mut(task_id)
            .ok_or_else(|| TaskError::NotFound(task_id.to_string()))?;

        task.state = TaskState::Failed;
        task.error_message = Some(error);
        task.completed_at = Some(Utc::now());
        Ok(())
    }

    fn get_task(&self, task_id: &str) -> Option<InferenceTask> {
        self.tasks.get(task_id).map(|t| t.clone())
    }

    fn get_tasks_by_session(&self, session_id: &str) -> Vec<InferenceTask> {
        self.session_tasks
            .get(session_id)
            .map(|task_ids| {
                task_ids
                    .iter()
                    .filter_map(|id| self.tasks.get(id).map(|t| t.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    fn cleanup_session(&self, session_id: &str) {
        if let Some((_, task_ids)) = self.session_tasks.remove(session_id) {
            for task_id in task_ids {
                self.tasks.remove(&task_id);
            }
            tracing::debug!(session_id = %session_id, "Session tasks cleaned up");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_task_lifecycle() {
        let (tx, mut rx) = mpsc::channel(100);
        let manager = InMemoryTaskManager::new(tx);

        let task = InferenceTask::new(
            "session-1".to_string(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            0,
            "Test content".to_string(),
        );
        let task_id = task.task_id.clone();

        // Submit
        let result = manager.submit(vec![task]);
        assert!(result.is_ok());
        let task_ids = result.unwrap();
        assert_eq!(task_ids.len(), 1);

        // Check queue
        let queued_id = rx.try_recv();
        assert!(queued_id.is_ok());
        assert_eq!(queued_id.unwrap(), task_id);

        // Get state
        let state = manager.get_state(&task_id);
        assert_eq!(state, Some(TaskState::Pending));

        // Set state
        let result = manager.set_state(&task_id, TaskState::Inferring);
        assert!(result.is_ok());
        assert_eq!(manager.get_state(&task_id), Some(TaskState::Inferring));

        // Cancel pending (should not cancel Inferring)
        let cancelled = manager.cancel_pending("session-1");
        assert_eq!(cancelled, 0);
    }

    #[tokio::test]
    async fn test_cancel_pending() {
        let (tx, _rx) = mpsc::channel(100);
        let manager = InMemoryTaskManager::new(tx);

        // Submit multiple tasks
        let tasks: Vec<InferenceTask> = (0..5)
            .map(|i| {
                InferenceTask::new(
                    "session-1".to_string(),
                    Uuid::new_v4(),
                    Uuid::new_v4(),
                    i,
                    format!("Content {}", i),
                )
            })
            .collect();

        manager.submit(tasks).unwrap();

        // Cancel all pending
        let cancelled = manager.cancel_pending("session-1");
        assert_eq!(cancelled, 5);

        // Verify all cancelled
        for task in manager.get_tasks_by_session("session-1") {
            assert_eq!(task.state, TaskState::Cancelled);
        }
    }
}
