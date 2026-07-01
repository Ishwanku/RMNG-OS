use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Persistent multi-agent session at `~/.rmng/sessions/<id>.json`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentSession {
    pub id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(default)]
    pub active_agents: HashMap<String, ActiveAgentSlot>,
    #[serde(default)]
    pub shared_context: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub task_state: TaskState,
    #[serde(default)]
    pub handoff_history: Vec<HandoffRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActiveAgentSlot {
    pub agent_id: String,
    pub layer: String,
    pub activated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct TaskState {
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub current_prompt: Option<String>,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HandoffRecord {
    pub timestamp: DateTime<Utc>,
    pub from_agent: String,
    pub from_layer: String,
    pub to_agent: String,
    pub to_layer: String,
    pub reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("session not found: {0}")]
    NotFound(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("parse error: {0}")]
    Parse(#[from] serde_json::Error),
}

pub struct SessionStore {
    root: PathBuf,
}

impl SessionStore {
    pub fn default_store() -> Self {
        Self::new(sessions_root())
    }

    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn create(&self) -> Result<AgentSession, SessionError> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now();
        let session = AgentSession {
            id: id.clone(),
            created_at: now,
            updated_at: now,
            active_agents: HashMap::new(),
            shared_context: HashMap::new(),
            task_state: TaskState {
                status: "open".into(),
                current_prompt: None,
                notes: Vec::new(),
            },
            handoff_history: Vec::new(),
        };
        self.save(&session)?;
        Ok(session)
    }

    pub fn load(&self, id: &str) -> Result<AgentSession, SessionError> {
        let path = self.path_for(id);
        if !path.is_file() {
            return Err(SessionError::NotFound(id.to_string()));
        }
        let raw = std::fs::read_to_string(&path)?;
        Ok(serde_json::from_str(&raw)?)
    }

    pub fn save(&self, session: &AgentSession) -> Result<(), SessionError> {
        std::fs::create_dir_all(&self.root)?;
        let path = self.path_for(&session.id);
        let raw = serde_json::to_string_pretty(session)?;
        std::fs::write(path, raw)?;
        Ok(())
    }

    pub fn list_ids(&self) -> Result<Vec<String>, SessionError> {
        if !self.root.is_dir() {
            return Ok(Vec::new());
        }
        let mut ids = Vec::new();
        for entry in std::fs::read_dir(&self.root)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "json") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    ids.push(stem.to_string());
                }
            }
        }
        ids.sort();
        Ok(ids)
    }

    pub fn set_active_agent(
        &self,
        session: &mut AgentSession,
        layer_key: &str,
        agent_id: &str,
        layer: &str,
    ) -> Result<(), SessionError> {
        session.active_agents.insert(
            layer_key.to_string(),
            ActiveAgentSlot {
                agent_id: agent_id.to_string(),
                layer: layer.to_string(),
                activated_at: Utc::now(),
            },
        );
        session.updated_at = Utc::now();
        self.save(session)
    }

    pub fn record_handoff(
        &self,
        session: &mut AgentSession,
        from_agent: &str,
        from_layer: &str,
        to_agent: &str,
        to_layer: &str,
        reason: &str,
        prompt: Option<&str>,
    ) -> Result<(), SessionError> {
        session.handoff_history.push(HandoffRecord {
            timestamp: Utc::now(),
            from_agent: from_agent.to_string(),
            from_layer: from_layer.to_string(),
            to_agent: to_agent.to_string(),
            to_layer: to_layer.to_string(),
            reason: reason.to_string(),
            prompt: prompt.map(str::to_string),
        });
        session.updated_at = Utc::now();
        self.save(session)
    }

    pub fn set_context(
        &self,
        session: &mut AgentSession,
        key: &str,
        value: serde_json::Value,
    ) -> Result<(), SessionError> {
        session.shared_context.insert(key.to_string(), value);
        session.updated_at = Utc::now();
        self.save(session)
    }

    fn path_for(&self, id: &str) -> PathBuf {
        self.root.join(format!("{id}.json"))
    }
}

pub fn sessions_root() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".rmng/sessions");
    }
    PathBuf::from(".rmng/sessions")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_load_and_handoff() {
        let dir = std::env::temp_dir().join(format!("rmng-session-test-{}", uuid::Uuid::new_v4()));
        let store = SessionStore::new(&dir);
        let mut session = store.create().expect("create");
        store
            .set_active_agent(&mut session, "L3", "repo-keeper", "L3")
            .expect("activate");
        store
            .record_handoff(
                &mut session,
                "swarm-coordinator",
                "L4",
                "repo-keeper",
                "L3",
                "delegate git workflow",
                Some("check status"),
            )
            .expect("handoff");
        let loaded = store.load(&session.id).expect("load");
        assert_eq!(loaded.handoff_history.len(), 1);
        assert_eq!(loaded.active_agents.get("L3").unwrap().agent_id, "repo-keeper");
        let _ = std::fs::remove_dir_all(dir);
    }
}
