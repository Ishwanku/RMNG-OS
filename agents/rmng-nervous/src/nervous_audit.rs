use rmng_core::{AuditCategory, AuditEntry, AuditLog, AuditTrack};

/// Append nervous-system events (LLM retries, handoffs) to the shared audit log.
pub fn log_nervous_event(action: &str, outcome: &str, detail: Option<&str>) {
    let category = if action.contains("circuit") {
        AuditCategory::Circuit
    } else if action.contains("fallback") || action.contains("llm") {
        AuditCategory::Llm
    } else if action.contains("handoff") {
        AuditCategory::Handoff
    } else {
        AuditCategory::Plan
    };
    emit_tracing(category, action, outcome, detail);
    let mut entry = AuditEntry::new(action, outcome);
    entry.category = Some(category);
    entry.track = Some(AuditTrack::Plan);
    entry.detail = detail.map(str::to_string);
    append_entry(&entry);
}

/// Record LLM telemetry to audit log (Sprint 9/10) — works with or without an active session.
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
    let mut entry = AuditEntry::new("nervous.llm_call", "success");
    entry.category = Some(AuditCategory::Llm);
    entry.track = Some(AuditTrack::Plan);
    entry.trace_id = session_id.map(str::to_string);
    entry.session_id = session_id.map(str::to_string);
    entry.agent_id = agent_id.map(str::to_string);
    entry.llm_profile = Some(profile_label.to_string());
    entry.duration_ms = Some(latency_ms);
    entry.tokens_prompt = prompt_tokens;
    entry.tokens_completion = completion_tokens;
    entry.cost_usd = estimated_cost_usd;
    entry.fallback_index = Some(fallback_index);
    entry.detail = Some(format!(
        "provider={provider} model={model} profile={profile_label}"
    ));
    append_entry(&entry);
}

/// Budget / governance events (Sprint 11).
pub fn log_system_event(action: &str, outcome: &str, detail: Option<&str>) {
    emit_tracing(AuditCategory::System, action, outcome, detail);
    let mut entry = AuditEntry::new(action, outcome);
    entry.category = Some(AuditCategory::System);
    entry.track = Some(AuditTrack::Plan);
    entry.detail = detail.map(str::to_string);
    append_entry(&entry);
}

fn emit_tracing(category: AuditCategory, action: &str, outcome: &str, detail: Option<&str>) {
    let detail = detail.unwrap_or("-");
    match category {
        AuditCategory::Circuit => {
            tracing::warn!(%action, %outcome, %detail, "circuit breaker");
        }
        AuditCategory::Handoff => {
            tracing::info!(%action, %outcome, %detail, "agent handoff");
        }
        AuditCategory::System if action.contains("budget") => {
            tracing::warn!(%action, %outcome, %detail, "budget");
        }
        AuditCategory::Llm if action.contains("fallback") => {
            tracing::warn!(%action, %outcome, %detail, "llm fallback");
        }
        _ => {
            tracing::debug!(%action, %outcome, %detail, "nervous event");
        }
    }
}

fn append_entry(entry: &AuditEntry) {
    let log = AuditLog::new(AuditLog::default_path());
    if let Err(e) = log.append(entry) {
        tracing::warn!(error = %e, "nervous audit write failed");
    }
}