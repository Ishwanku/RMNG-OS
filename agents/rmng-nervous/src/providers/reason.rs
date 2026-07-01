use super::factory::LlmBackend;
use super::prompt::build_reasoning_prompt;
use super::types::{parse_core_intent, LlmReasonContext, LlmRequest, ProviderError};
use crate::nervous_audit::log_nervous_event;
use rmng_core::CoreIntent;

const REPAIR_SUFFIX: &str = r#"
Your previous response was NOT valid core-intent v2 JSON.
Return ONLY a single JSON object with top-level "action" (tool.execute, mcp.proxy, or plan.only).
Include metadata.session_id if a session is active. No markdown fences, no prose."#;

/// Reason once, then auto-retry once on invalid JSON (Sprint 6).
pub async fn reason_with_retry(
    backend: &LlmBackend,
    provider_id: &str,
    assembled: &str,
    ctx: &LlmReasonContext<'_>,
) -> Result<CoreIntent, ProviderError> {
    let req = LlmRequest {
        assembled_prompt: assembled,
        ctx: ctx.clone(),
    };
    match backend.complete(req).await {
        Ok(resp) => match parse_core_intent(&resp.content) {
            Ok(intent) => Ok(intent),
            Err(e) => attempt_repair(backend, provider_id, assembled, ctx, &resp.content, e).await,
        },
        Err(e) => Err(e),
    }
}

async fn attempt_repair(
    backend: &LlmBackend,
    provider_id: &str,
    assembled: &str,
    ctx: &LlmReasonContext<'_>,
    bad_output: &str,
    parse_err: ProviderError,
) -> Result<CoreIntent, ProviderError> {
    let preview: String = bad_output.chars().take(200).collect();
    log_nervous_event(
        "nervous.llm_retry",
        "retry",
        Some(&format!(
            "provider={provider_id} parse_error={parse_err} preview={preview}"
        )),
    );
    let repair_prompt = format!(
        "{}\n\n{REPAIR_SUFFIX}",
        build_reasoning_prompt(assembled, ctx)
    );
    let repair_req = LlmRequest {
        assembled_prompt: &repair_prompt,
        ctx: ctx.clone(),
    };
    let resp = backend.complete(repair_req).await?;
    parse_core_intent(&resp.content).map_err(|e| {
        log_nervous_event(
            "nervous.llm_retry",
            "failed",
            Some(&format!("provider={provider_id} repair_failed={e}")),
        );
        e
    })
}