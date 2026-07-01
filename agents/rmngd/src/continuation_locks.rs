//! Per-session locks — only one continue loop per session (Sprint 27).

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, OwnedMutexGuard};

/// Serializes `continue_session` / background continuation per session id.
pub struct SessionContinuationLocks {
    sessions: Mutex<HashMap<String, Arc<Mutex<()>>>>,
}

impl SessionContinuationLocks {
    pub fn new() -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
        }
    }

    async fn session_mutex(&self, session_id: &str) -> Arc<Mutex<()>> {
        let mut map = self.sessions.lock().await;
        map.entry(session_id.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    pub async fn is_busy(&self, session_id: &str) -> bool {
        let m = self.session_mutex(session_id).await;
        let busy = m.try_lock().is_err();
        busy
    }

    /// Block until this session's continuation slot is free.
    pub async fn acquire_owned(&self, session_id: &str) -> OwnedMutexGuard<()> {
        let m = self.session_mutex(session_id).await;
        m.lock_owned().await
    }

    /// Returns `None` when another continuation task holds the session lock.
    pub async fn try_acquire_owned(&self, session_id: &str) -> Option<OwnedMutexGuard<()>> {
        let m = self.session_mutex(session_id).await;
        m.try_lock_owned().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn second_acquire_fails_while_held() {
        let locks = SessionContinuationLocks::new();
        let _g1 = locks.try_acquire_owned("sid").await.expect("first");
        assert!(locks.is_busy("sid").await);
        assert!(locks.try_acquire_owned("sid").await.is_none());
    }
}
