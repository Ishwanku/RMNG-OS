use crate::intent::CoreIntent;
use crate::response::HandleResponse;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Max characters stored per tool output in session shared context.
pub const MAX_TOOL_OUTPUT_LEN: usize = 4096;
/// Max tool result records retained per session.
const MAX_TOOL_RESULTS: usize = 50;
/// Max LLM call records retained per session.
const MAX_LLM_CALLS: usize = 100;

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
    /// Per-agent LLM call metrics (Sprint 8).
    #[serde(default)]
    pub llm_calls: Vec<LlmCallRecord>,
}

/// One nervous-system LLM invocation recorded on the session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LlmCallRecord {
    pub timestamp: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
    pub provider: String,
    pub model: String,
    pub profile_label: String,
    pub latency_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_cost_usd: Option<f64>,
    #[serde(default)]
    pub fallback_index: u32,
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

/// Tool execution result written back into session shared context (Sprint 4b).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolResultRecord {
    pub timestamp: DateTime<Utc>,
    pub tool: String,
    pub parameters: serde_json::Value,
    pub output: String,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub handoff_from: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peak_rss_kb: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_time_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime_ms: Option<u64>,
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

/// Default session TTL in days (ADR-018). Override with `RMNG_SESSION_TTL_DAYS`.
pub const DEFAULT_SESSION_TTL_DAYS: u32 = 90;

#[derive(Debug, Clone, Copy)]
pub struct SessionLoadOptions {
    /// When true, delete and reject sessions older than TTL on load.
    pub enforce_ttl: bool,
}

impl Default for SessionLoadOptions {
    fn default() -> Self {
        Self { enforce_ttl: true }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("session not found: {0}")]
    NotFound(String),
    #[error("session expired (TTL): {0}")]
    Expired(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("parse error: {0}")]
    Parse(#[from] serde_json::Error),
}

#[derive(Clone)]
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
            llm_calls: Vec::new(),
        };
        self.save(&session)?;
        Ok(session)
    }

    pub fn load(&self, id: &str) -> Result<AgentSession, SessionError> {
        self.load_with_options(id, SessionLoadOptions::default())
    }

    pub fn load_with_options(
        &self,
        id: &str,
        options: SessionLoadOptions,
    ) -> Result<AgentSession, SessionError> {
        let path = self.path_for(id);
        if !path.is_file() {
            return Err(SessionError::NotFound(id.to_string()));
        }
        let raw = std::fs::read_to_string(&path)?;
        let mut session: AgentSession = serde_json::from_str(&raw)?;
        if options.enforce_ttl {
            if let Some(ttl_days) = session_ttl_days() {
                let cutoff = Utc::now() - chrono::Duration::days(ttl_days as i64);
                if session.updated_at < cutoff {
                    let _ = std::fs::remove_file(&path);
                    return Err(SessionError::Expired(id.to_string()));
                }
            }
        }
        session.refresh_lifecycle();
        Ok(session)
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


    /// Persist multi-hop orchestration progress in `shared_context.orchestration` (Sprint 23).
    pub fn set_orchestration_state(
        &self,
        session: &mut AgentSession,
        state: serde_json::Value,
    ) -> Result<(), SessionError> {
        session
            .shared_context
            .insert("orchestration".to_string(), state);
        session.updated_at = Utc::now();
        self.save(session)
    }

    pub fn record_chain_failure(
        &self,
        session: &mut AgentSession,
        hop_index: usize,
        from_agent: &str,
        to_agent: &str,
        error: &str,
    ) -> Result<(), SessionError> {
        if let Some(orch) = session.shared_context.get_mut("orchestration") {
            if let Some(obj) = orch.as_object_mut() {
                obj.insert("status".into(), serde_json::json!("failed"));
                obj.insert("failed_hop".into(), serde_json::json!(hop_index));
                obj.insert("failed_from".into(), serde_json::json!(from_agent));
                obj.insert("failed_to".into(), serde_json::json!(to_agent));
                obj.insert("error".into(), serde_json::json!(error));
            }
        } else {
            session.shared_context.insert(
                "orchestration".to_string(),
                serde_json::json!({
                    "status": "failed",
                    "failed_hop": hop_index,
                    "failed_from": from_agent,
                    "failed_to": to_agent,
                    "error": error,
                }),
            );
        }
        session.updated_at = Utc::now();
        self.save(session)
    }

    pub fn clear_orchestration_state(&self, session: &mut AgentSession) -> Result<(), SessionError> {
        session.shared_context.remove("orchestration");
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

    pub fn record_llm_call(
        &self,
        session: &mut AgentSession,
        record: LlmCallRecord,
    ) -> Result<(), SessionError> {
        session.llm_calls.push(record);
        if session.llm_calls.len() > MAX_LLM_CALLS {
            let drain = session.llm_calls.len() - MAX_LLM_CALLS;
            session.llm_calls.drain(0..drain);
        }
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

    /// Append a tool/MCP dispatch result to `shared_context.tool_results`.
    pub fn record_tool_result(
        &self,
        session: &mut AgentSession,
        record: ToolResultRecord,
    ) -> Result<(), SessionError> {
        let entry = session
            .shared_context
            .entry("tool_results".to_string())
            .or_insert_with(|| serde_json::Value::Array(Vec::new()));
        if let Some(list) = entry.as_array_mut() {
            list.push(serde_json::to_value(&record)?);
            while list.len() > MAX_TOOL_RESULTS {
                list.remove(0);
            }
        }
        session.updated_at = Utc::now();
        self.save(session)
    }

    /// Remove sessions with `updated_at` older than `days`. Returns removed ids.
    pub fn prune_older_than(&self, days: u32, dry_run: bool) -> Result<Vec<String>, SessionError> {
        let cutoff = Utc::now() - chrono::Duration::days(days as i64);
        let mut removed = Vec::new();
        for id in self.list_ids()? {
            let session = self.load(&id)?;
            if session.updated_at < cutoff {
                removed.push(id.clone());
                if !dry_run {
                    std::fs::remove_file(self.path_for(&id))?;
                }
            }
        }
        Ok(removed)
    }

    fn path_for(&self, id: &str) -> PathBuf {
        self.root.join(format!("{id}.json"))
    }
}


impl AgentSession {
    /// Mark session as actively processing a prompt (called on ask/handoff).
    pub fn mark_active(&mut self, prompt: &str) {
        self.task_state.current_prompt = Some(prompt.to_string());
        self.task_state.status = "active".into();
        self.updated_at = Utc::now();
    }

    /// Update `task_state.status` from `updated_at`: active (<1h), idle (<7d), stale (≥7d).
    pub fn refresh_lifecycle(&mut self) {
        let hours = (Utc::now() - self.updated_at).num_hours();
        self.task_state.status = if hours < 1 {
            "active".into()
        } else if hours < 24 * 7 {
            "idle".into()
        } else {
            "stale".into()
        };
    }

    /// Lifecycle label for CLI listing (`active` / `idle` / `stale`).
    pub fn lifecycle_label(&self) -> &str {
        match self.task_state.status.as_str() {
            "active" | "idle" | "stale" => &self.task_state.status,
            _ => "open",
        }
    }

    /// `active` if updated within `active_within_days`, else `stale`.
    pub fn freshness_label(&self, active_within_days: i64) -> &'static str {
        let cutoff = Utc::now() - chrono::Duration::days(active_within_days);
        if self.updated_at >= cutoff {
            "active"
        } else {
            "stale"
        }
    }

    /// Human-readable summary of recent tool results for live LLM prompts.
    pub fn tool_results_summary(&self, max_entries: usize) -> String {
        let Some(arr) = self
            .shared_context
            .get("tool_results")
            .and_then(|v| v.as_array())
        else {
            return "(no prior tool results)".into();
        };
        if arr.is_empty() {
            return "(no prior tool results)".into();
        }
        let tail: Vec<_> = arr.iter().rev().take(max_entries).collect();
        tail.into_iter()
            .rev()
            .map(|entry| {
                let tool = entry["tool"].as_str().unwrap_or("unknown");
                let ok = entry["success"].as_bool().unwrap_or(false);
                let status = if ok { "ok" } else { "failed" };
                let output = entry["output"].as_str().unwrap_or("");
                let preview: String = output.chars().take(500).collect();
                format!("- {tool} [{status}]: {preview}")
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Format shared context + recent handoffs for nervous-system prompt injection.
    pub fn prompt_context(&self) -> String {
        let mut parts = Vec::new();
        parts.push(format!("session_id: {}", self.id));
        parts.push(format!("lifecycle: {}", self.lifecycle_label()));
        if let Some(prompt) = &self.task_state.current_prompt {
            parts.push(format!("current_task: {prompt}"));
        }
        let tool_summary = self.tool_results_summary(5);
        if tool_summary != "(no prior tool results)" {
            parts.push(format!("recent_tool_results:\n{tool_summary}"));
        }
        let other_ctx: HashMap<_, _> = self
            .shared_context
            .iter()
            .filter(|(k, _)| k.as_str() != "tool_results")
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        if !other_ctx.is_empty() {
            let ctx = serde_json::to_string_pretty(&other_ctx).unwrap_or_else(|_| "{}".into());
            parts.push(format!("shared_context:\n{ctx}"));
        }
        if let Some(orch) = self.shared_context.get("orchestration") {
            parts.push(format!(
                "orchestration_chain: {}",
                serde_json::to_string(orch).unwrap_or_else(|_| "{}".into())
            ));
        }
        if !self.handoff_history.is_empty() {
            let tail: Vec<_> = self.handoff_history.iter().rev().take(5).collect();
            let lines: Vec<String> = tail
                .into_iter()
                .rev()
                .map(|h| {
                    format!(
                        "- {} ({}) → {} ({}): {}",
                        h.from_agent, h.from_layer, h.to_agent, h.to_layer, h.reason
                    )
                })
                .collect();
            parts.push(format!("recent_handoffs:
{}", lines.join("
")));
        }
        if !self.active_agents.is_empty() {
            let active: Vec<String> = self
                .active_agents
                .iter()
                .map(|(layer, slot)| format!("{layer}: {}", slot.agent_id))
                .collect();
            parts.push(format!("active_agents: {}", active.join(", ")));
        }
        parts.join("
")
    }
}

/// TTL days from `RMNG_SESSION_TTL_DAYS`, or default. Set to `0` to disable expiry on load.
pub fn session_ttl_days() -> Option<u32> {
    match std::env::var("RMNG_SESSION_TTL_DAYS") {
        Ok(raw) => {
            let days: u32 = raw.parse().ok()?;
            if days == 0 {
                None
            } else {
                Some(days)
            }
        }
        Err(_) => Some(DEFAULT_SESSION_TTL_DAYS),
    }
}

pub fn sessions_root() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".rmng/sessions");
    }
    PathBuf::from(".rmng/sessions")
}

/// Build a session write-back record from a dispatched intent and rmngd response.
pub fn build_tool_result_record(
    intent: &CoreIntent,
    resp: &HandleResponse,
) -> Option<ToolResultRecord> {
    let (tool, parameters) = match intent {
        CoreIntent::ToolExecute {
            target, parameters, ..
        } => (target.clone(), parameters.clone()),
        CoreIntent::McpProxy {
            mcp_server,
            mcp_tool,
            mcp_args,
            ..
        } => (
            format!("{mcp_server}.{mcp_tool}"),
            mcp_args.clone(),
        ),
        CoreIntent::PlanOnly { .. } => return None,
    };
    let handoff_from = intent
        .metadata()
        .and_then(|m| m.handoff_from.clone());
    let (output, success, exit_code, peak_rss_kb, cpu_time_ms, runtime_ms) = if let Some(result) = &resp.tool_result {
        let mut out = result.output.clone();
        if out.len() > MAX_TOOL_OUTPUT_LEN {
            out.truncate(MAX_TOOL_OUTPUT_LEN);
            out.push_str("\n...(truncated)");
        }
        (
            out,
            resp.ok && result.success,
            result.exit_code,
            result.resources.as_ref().and_then(|r| r.peak_rss_kb),
            result.resources.as_ref().and_then(|r| r.cpu_time_ms),
            result.resources.as_ref().and_then(|r| r.runtime_ms),
        )
    } else if resp.ok {
        (
            resp.action
                .clone()
                .unwrap_or_else(|| "ok".into()),
            true,
            None,
            None,
            None,
            None,
        )
    } else {
        (
            resp.error
                .clone()
                .unwrap_or_else(|| "unknown error".into()),
            false,
            None,
            None,
            None,
            None,
        )
    };
    Some(ToolResultRecord {
        timestamp: Utc::now(),
        tool,
        parameters,
        output,
        success,
        exit_code,
        handoff_from,
        peak_rss_kb,
        cpu_time_ms,
        runtime_ms,
    })
}

/// After rmngd dispatch, persist tool output into the session when `--session` is active.
pub fn persist_dispatch_to_session(
    store: &SessionStore,
    session_id: &str,
    intent: &CoreIntent,
    resp: &HandleResponse,
) -> Result<(), SessionError> {
    let Some(record) = build_tool_result_record(intent, resp) else {
        return Ok(());
    };
    let mut session = store.load(session_id)?;
    store.record_tool_result(&mut session, record)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_tool_result_in_shared_context() {
        let dir = std::env::temp_dir().join(format!("rmng-tool-ctx-{}", uuid::Uuid::new_v4()));
        let store = SessionStore::new(&dir);
        let session = store.create().expect("create");
        let intent = CoreIntent::ToolExecute {
            target: "git.status".into(),
            parameters: serde_json::json!({}),
            metadata: None,
        };
        let resp = HandleResponse::core_success(
            "tool.execute:git.status",
            Some(crate::tool::ToolResult {
                success: true,
                output: "branch main".into(),
                exit_code: Some(0),
                resources: None,
            }),
        );
        persist_dispatch_to_session(&store, &session.id, &intent, &resp).expect("persist");
        let loaded = store.load(&session.id).expect("load");
        let results = loaded
            .shared_context
            .get("tool_results")
            .and_then(|v| v.as_array())
            .expect("tool_results array");
        assert_eq!(results.len(), 1);
        assert!(results[0]["output"].as_str().unwrap().contains("branch main"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn prune_removes_old_sessions() {
        let dir = std::env::temp_dir().join(format!("rmng-prune-{}", uuid::Uuid::new_v4()));
        let store = SessionStore::new(&dir);
        let mut session = store.create().expect("create");
        session.updated_at = Utc::now() - chrono::Duration::days(60);
        store.save(&session).expect("save stale");
        let fresh = store.create().expect("create fresh");
        let removed = store.prune_older_than(30, false).expect("prune");
        assert_eq!(removed.len(), 1);
        assert_eq!(removed[0], session.id);
        assert!(store.load(&fresh.id).is_ok());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn ttl_expired_session_removed_on_load() {
        let dir = std::env::temp_dir().join(format!("rmng-ttl-{}", uuid::Uuid::new_v4()));
        let store = SessionStore::new(&dir);
        let mut session = store.create().expect("create");
        session.updated_at = Utc::now() - chrono::Duration::days(120);
        store.save(&session).expect("save old");
        std::env::set_var("RMNG_SESSION_TTL_DAYS", "90");
        let err = store.load(&session.id).expect_err("expired");
        assert!(matches!(err, SessionError::Expired(_)));
        assert!(store.load(&session.id).is_err());
        std::env::remove_var("RMNG_SESSION_TTL_DAYS");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn tool_results_summary_formats_for_llm() {
        let dir = std::env::temp_dir().join(format!("rmng-summary-{}", uuid::Uuid::new_v4()));
        let store = SessionStore::new(&dir);
        let mut session = store.create().expect("create");
        store
            .record_tool_result(
                &mut session,
                ToolResultRecord {
                    timestamp: Utc::now(),
                    tool: "git.status".into(),
                    parameters: serde_json::json!({}),
                    output: "On branch main".into(),
                    success: true,
                    exit_code: Some(0),
                    handoff_from: None,
                    peak_rss_kb: None,
                    cpu_time_ms: None,
                    runtime_ms: None,
                },
            )
            .expect("record");
        let summary = session.tool_results_summary(3);
        assert!(summary.contains("git.status"));
        assert!(summary.contains("On branch main"));
        let _ = std::fs::remove_dir_all(dir);
    }

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
