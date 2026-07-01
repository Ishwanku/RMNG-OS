//! Daemon IPC envelopes beyond core intents (Sprint 26).

use crate::RmngError;
use serde::{Deserialize, Serialize};

/// Line sent to rmngd — either a core intent or an orchestration control action.
#[derive(Debug, Clone)]
pub enum DaemonLine {
    Intent(crate::IncomingIntent),
    OrchestrationContinue { session_id: String },
}

#[derive(Debug, Deserialize)]
struct RawDaemonLine {
    action: String,
    #[serde(default)]
    session_id: Option<String>,
}

/// Parse one JSON line for rmngd (core intent v2/v1 or orchestration.continue).
pub fn parse_daemon_line(json: &str) -> Result<DaemonLine, RmngError> {
    let raw: RawDaemonLine = serde_json::from_str(json)?;
    if raw.action == "orchestration.continue" {
        let session_id = raw.session_id.ok_or_else(|| {
            RmngError::InvalidIntent("orchestration.continue requires session_id".into())
        })?;
        if session_id.trim().is_empty() {
            return Err(RmngError::InvalidIntent(
                "orchestration.continue session_id must not be empty".into(),
            ));
        }
        return Ok(DaemonLine::OrchestrationContinue {
            session_id: session_id.trim().to_string(),
        });
    }
    Ok(DaemonLine::Intent(crate::parse_incoming(json)?))
}

/// Response for orchestration.continue IPC.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationContinueResponse {
    pub ok: bool,
    pub action: String,
    pub session_id: String,
    pub steps_run: u32,
    pub finished: bool,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dispatch_actions: Vec<String>,
}

impl OrchestrationContinueResponse {
    pub fn success(
        session_id: &str,
        steps_run: u32,
        finished: bool,
        status: &str,
        dispatch_actions: Vec<String>,
    ) -> Self {
        Self {
            ok: true,
            action: "orchestration.continue".into(),
            session_id: session_id.to_string(),
            steps_run,
            finished,
            status: status.to_string(),
            error: None,
            dispatch_actions,
        }
    }

    pub fn failure(session_id: &str, error: impl Into<String>) -> Self {
        Self {
            ok: false,
            action: "orchestration.continue".into(),
            session_id: session_id.to_string(),
            steps_run: 0,
            finished: true,
            status: "failed".into(),
            error: Some(error.into()),
            dispatch_actions: Vec::new(),
        }
    }

    pub fn timed_out(session_id: &str, timeout_secs: u64) -> Self {
        Self {
            ok: false,
            action: "orchestration.continue".into(),
            session_id: session_id.to_string(),
            steps_run: 0,
            finished: true,
            status: "timed_out".into(),
            error: Some(format!("auto-continue timed out after {timeout_secs}s")),
            dispatch_actions: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_orchestration_continue() {
        let json = r#"{"action":"orchestration.continue","session_id":"abc-123"}"#;
        let line = parse_daemon_line(json).expect("parse");
        match line {
            DaemonLine::OrchestrationContinue { session_id } => {
                assert_eq!(session_id, "abc-123");
            }
            _ => panic!("expected continue"),
        }
    }

    #[test]
    fn still_parses_core_intent() {
        let json = r#"{"action":"tool.execute","target":"git.status","parameters":{}}"#;
        let line = parse_daemon_line(json).expect("parse");
        assert!(matches!(line, DaemonLine::Intent(_)));
    }
}
