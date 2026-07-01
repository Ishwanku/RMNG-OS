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
    append_entry(&entry);
}

/// Record LLM telemetry to audit log (Sprint 9) — works with or without an active session.
pub fn log_llm_telemetry(
    provider: &str,
    model: &str,
    profile_label: &str,
    agent_id: Option<&str>,
    session_id: Option<&str>,
    latency_ms: u64,
    prompt_tokens: Option<u32>,
    completion_tokens: Option<u32>,
    estimated_cost_usd: Option<f64>,
    fallback_index: u32,
) {
    let tokens = match (prompt_tokens, completion_tokens) {
        (Some(p), Some(c)) => format!("prompt={p} completion={c}"),
        (Some(p), None) => format!("prompt={p}"),
        (None, Some(c)) => format!("completion={c}"),
        _ => "tokens=unknown".into(),
    };
    let cost = estimated_cost_usd
        .map(|c| format!(" cost_usd={c:.6}"))
        .unwrap_or_default();
    let agent = agent_id.unwrap_or("-");
    let session = session_id.unwrap_or("-");
    let fallback = if fallback_index > 0 {
        format!(" fallback_index={fallback_index}")
    } else {
        String::new()
    };
    let detail = format!(
        "provider={provider} model={model} profile={profile_label} agent={agent} session={session} {tokens}{cost}{fallback} latency_ms={latency_ms}"
    );
    let entry = AuditEntry {
        timestamp: Utc::now(),
        intent_id: Uuid::new_v4(),
        trace_id: session_id.map(str::to_string),
        skill_name: None,
        track: Some(AuditTrack::Plan),
        duration_ms: Some(latency_ms),
        mcp_server: None,
        action: "nervous.llm_call".into(),
        outcome: "success".into(),
        detail: Some(detail),
    };
    append_entry(&entry);
}

fn append_entry(entry: &AuditEntry) {
    let log = AuditLog::new(AuditLog::default_path());
    if let Err(e) = log.append(entry) {
        tracing::warn!(error = %e, "nervous audit write failed");
    }
}