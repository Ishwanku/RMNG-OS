use rmng_core::{AuditEntry, AuditLog, AuditTrack};
use chrono::Utc;
use uuid::Uuid;

/// Append nervous-system events (LLM retries, handoffs) to the shared audit log.
pub fn log_nervous_event(action: &str, outcome: &str, detail: Option<&str>) {
    let entry = AuditEntry {
        timestamp: Utc::now(),
        intent_id: Uuid::new_v4(),
        trace_id: None,
        skill_name: None,
        track: Some(AuditTrack::Plan),
        duration_ms: None,
        mcp_server: None,
        action: action.to_string(),
        outcome: outcome.to_string(),
        detail: detail.map(str::to_string),
    };
    let log = AuditLog::new(AuditLog::default_path());
    if let Err(e) = log.append(&entry) {
        tracing::warn!(error = %e, "nervous audit write failed");
    }
}