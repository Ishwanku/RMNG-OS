use super::backoff::retry_delay;
use super::cost::enrich_usage_cost;
use super::factory::LlmBackend;
use super::prompt::build_reasoning_prompt;
use super::types::{parse_core_intent, LlmReasonContext, LlmRequest, LlmUsage, ProviderError, ReasonResult};
use crate::nervous_audit::log_nervous_event;
const REPAIR_SUFFIX: &str = r#"
Your previous response was NOT valid core-intent v2 JSON.
Return ONLY a single JSON object with top-level "action" (tool.execute, mcp.proxy, or plan.only).
Include metadata.session_id if a session is active. No markdown fences, no prose."#;

/// Reason once, then auto-retry once on invalid JSON (Sprint 6). Aggregates token usage (Sprint 9).
pub async fn reason_with_retry(
    backend: &LlmBackend,
    provider_id: &str,
    assembled: &str,
    ctx: &LlmReasonContext<'_>,
) -> Result<ReasonResult, ProviderError> {
    let req = LlmRequest {
        assembled_prompt: assembled,
        ctx: ctx.clone(),
    };
    let resp = backend.complete(req).await?;
    let mut usage = resp.usage.clone();
    enrich_usage_cost(resp.provider_id, &resp.model, &mut usage);

    match parse_core_intent(&resp.content) {
        Ok(intent) => Ok(ReasonResult { intent, usage }),
        Err(e) => {
            attempt_repair(backend, provider_id, assembled, ctx, &resp.content, e, usage).await
        }
    }
}

async fn attempt_repair(
    backend: &LlmBackend,
    provider_id: &str,
    assembled: &str,
    ctx: &LlmReasonContext<'_>,
    bad_output: &str,
    parse_err: ProviderError,
    mut prior_usage: LlmUsage,
) -> Result<ReasonResult, ProviderError> {
    let preview: String = bad_output.chars().take(200).collect();
    log_nervous_event(
        "nervous.llm_retry",
        "retry",
        Some(&format!(
            "provider={provider_id} parse_error={parse_err} preview={preview}"
        )),
    );
    tokio::time::sleep(retry_delay(0)).await;
    let repair_prompt = format!(
        "{}\n\n{REPAIR_SUFFIX}",
        build_reasoning_prompt(assembled, ctx)
    );
    let repair_req = LlmRequest {
        assembled_prompt: &repair_prompt,
        ctx: ctx.clone(),
    };
    let resp = backend.complete(repair_req).await?;
    let mut repair_usage = resp.usage.clone();
    enrich_usage_cost(resp.provider_id, &resp.model, &mut repair_usage);
    prior_usage.merge(&repair_usage);

    parse_core_intent(&resp.content)
        .map(|intent| ReasonResult {
            intent,
            usage: prior_usage,
        })
        .map_err(|e| {
            log_nervous_event(
                "nervous.llm_retry",
                "failed",
                Some(&format!("provider={provider_id} repair_failed={e}")),
            );
            e
        })
}