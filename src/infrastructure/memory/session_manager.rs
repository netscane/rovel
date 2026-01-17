//! In-Memory Session Manager Implementation

use chrono::Utc;
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::application::ports::{Session, SessionError, SessionManagerPort};

/// 内存会话管理器
pub struct InMemorySessionManager {
    sessions: DashMap<String, Session>,
}

impl InMemorySessionManager {
    pub fn new() -> Self {
        Self {
            sessions: DashMap::new(),
        }
    }

    pub fn arc(self) -> Arc<Self> {
        Arc::new(self)
    }
}

impl Default for InMemorySessionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionManagerPort for InMemorySessionManager {
    fn create(&self, session: Session) -> Result<String, SessionError> {
        let session_id = session.id.clone();
        if self.sessions.contains_key(&session_id) {
            return Err(SessionError::AlreadyExists(session_id));
        }
        self.sessions.insert(session_id.clone(), session);
        tracing::info!(session_id = %session_id, "Session created");
        Ok(session_id)
    }

    fn get(&self, id: &str) -> Result<Session, SessionError> {
        self.sessions
            .get(id)
            .map(|s| s.clone())
            .ok_or_else(|| SessionError::NotFound(id.to_string()))
    }

    fn update_index(&self, id: &str, index: u32) -> Result<(), SessionError> {
        let mut session = self
            .sessions
            .get_mut(id)
            .ok_or_else(|| SessionError::NotFound(id.to_string()))?;
        session.current_index = index;
        session.last_activity = Utc::now();
        tracing::debug!(session_id = %id, index = index, "Session index updated");
        Ok(())
    }

    fn update_voice(&self, id: &str, voice_id: Uuid) -> Result<(), SessionError> {
        let mut session = self
            .sessions
            .get_mut(id)
            .ok_or_else(|| SessionError::NotFound(id.to_string()))?;
        session.voice_id = voice_id;
        session.last_activity = Utc::now();
        tracing::debug!(session_id = %id, voice_id = %voice_id, "Session voice updated");
        Ok(())
    }

    fn is_valid(&self, id: &str) -> bool {
        self.sessions.contains_key(id)
    }

    fn close(&self, id: &str) -> Result<(), SessionError> {
        self.sessions
            .remove(id)
            .map(|_| {
                tracing::info!(session_id = %id, "Session closed");
            })
            .ok_or_else(|| SessionError::NotFound(id.to_string()))
    }

    fn touch(&self, id: &str) {
        if let Some(mut session) = self.sessions.get_mut(id) {
            session.last_activity = Utc::now();
        }
    }

    fn get_expired_sessions(&self, idle_timeout_secs: u64) -> Vec<String> {
        let now = Utc::now();
        let timeout = chrono::Duration::seconds(idle_timeout_secs as i64);

        self.sessions
            .iter()
            .filter_map(|entry| {
                let elapsed = now - entry.last_activity;
                if elapsed > timeout {
                    Some(entry.key().clone())
                } else {
                    None
                }
            })
            .collect()
    }

    fn list_all(&self) -> Vec<String> {
        self.sessions.iter().map(|e| e.key().clone()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_lifecycle() {
        let manager = InMemorySessionManager::new();
        let session = Session::new(Uuid::new_v4(), Uuid::new_v4(), 0);
        let session_id = session.id.clone();

        // Create
        let result = manager.create(session);
        assert!(result.is_ok());

        // Get
        let session = manager.get(&session_id);
        assert!(session.is_ok());
        assert_eq!(session.unwrap().current_index, 0);

        // Update index
        let result = manager.update_index(&session_id, 10);
        assert!(result.is_ok());
        let session = manager.get(&session_id).unwrap();
        assert_eq!(session.current_index, 10);

        // Is valid
        assert!(manager.is_valid(&session_id));

        // Close
        let result = manager.close(&session_id);
        assert!(result.is_ok());
        assert!(!manager.is_valid(&session_id));
    }
}
