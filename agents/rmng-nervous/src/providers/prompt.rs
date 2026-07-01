use super::types::LlmReasonContext;

const CHAIN_FORMAT_RULES: &str = r#"
CRITICAL handoff_chain rules:
- MUST be a JSON array: ["swarm-coordinator","repo-keeper"] NOT a comma string.
- NOT arrow syntax in JSON (no "a -> b" strings).
- First agent MUST match your current agent id when you start the chain.
- Optional: hop_failure_policy ("retry"|"skip"|"abort"), hop_retry_max (integer).
"#;

const INTENT_EXAMPLES: &str = r#"
Example intents (emit exactly one JSON object):
{"action":"tool.execute","target":"git.status","parameters":{},"metadata":{"session_id":"<sid>"}}
{"action":"mcp.proxy","mcp_server":"github","mcp_tool":"search_issues","mcp_args":{"query":"repo:Ishwanku/RMNG-OS is:open"},"metadata":{"session_id":"<sid>"}}
{"action":"plan.only","reasoning":"Task complete. Summarize prior tool results.","metadata":{"session_id":"<sid>"}}
{"action":"plan.only","reasoning":"Delegate via chain.","metadata":{"session_id":"<sid>","handoff_chain":["swarm-coordinator","repo-keeper","runtime-executor"]}}
{"action":"plan.only","reasoning":"Delegate to specialist.","metadata":{"session_id":"<sid>","handoff_to":"repo-keeper"}}
{"action":"plan.only","reasoning":"Specialist done; return summary.","metadata":{"session_id":"<sid>","handoff_return_to":"swarm-coordinator"}}
{"action":"plan.only","reasoning":"Git hygiene needs repo then executor.","metadata":{"session_id":"<sid>","handoff_chain":["swarm-coordinator","repo-keeper","runtime-executor"],"chain_id":"<sid>","hop_failure_policy":"skip"}}
"#;

fn provider_chain_hints(provider_id: Option<&str>) -> &'static str {
    let Some(id) = provider_id else {
        return "";
    };
    if id.eq_ignore_ascii_case("groq") {
        "Groq: use strict JSON array for handoff_chain; avoid trailing commas."
    } else if id.eq_ignore_ascii_case("grok") || id.eq_ignore_ascii_case("xai") {
        "Grok/xAI: never emit handoff_chain as comma-separated string - JSON array only."
    } else if id.eq_ignore_ascii_case("ollama") {
        "Ollama: keep JSON compact; handoff_chain as array; no markdown fences."
    } else if id.eq_ignore_ascii_case("openai")
        || id.eq_ignore_ascii_case("anthropic")
        || id.eq_ignore_ascii_case("google")
    {
        "Emit raw JSON only; handoff_chain must be an array of agent id strings."
    } else {
        ""
    }
}

/// Build the final prompt sent to any LLM provider (shared across adapters).
pub fn build_reasoning_prompt(assembled: &str, ctx: &LlmReasonContext<'_>) -> String {
    let mut hints = Vec::new();
    if let Some(name) = ctx.skill_name {
        hints.push(format!(
            "Include metadata.skill_name = \"{name}\" when appropriate."
        ));
    }
    if let Some(sid) = ctx.session_id {
        hints.push(format!(
            "REQUIRED: include metadata.session_id = \"{sid}\" on the intent."
        ));
    }
    if let Some(agent) = ctx.agent_id {
        hints.push(format!(
            "You are agent \"{agent}\". Only emit tools listed in your Allowed tools section."
        ));
    }
    let hint_block = if hints.is_empty() {
        String::new()
    } else {
        format!("\n{}\n", hints.join("\n"))
    };
    let provider_hint = provider_chain_hints(ctx.provider_id);
    let provider_block = if provider_hint.is_empty() {
        String::new()
    } else {
        format!("\nProvider note: {provider_hint}\n")
    };
    format!(
        "{assembled}{hint_block}{provider_block}{CHAIN_FORMAT_RULES}{INTENT_EXAMPLES}\nHandoff fields (plan.only only): handoff_to (string), handoff_chain (JSON array of strings), handoff_return_to (string), chain_id (string), hop_failure_policy (retry|skip|abort), hop_retry_max (integer).
Respond with a single JSON object for core-intent v2. No markdown fences, no prose outside JSON."
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn includes_chain_examples() {
        let ctx = LlmReasonContext::default();
        let out = build_reasoning_prompt("## User request\ntest", &ctx);
        assert!(out.contains("handoff_chain"));
        assert!(out.contains("handoff_return_to"));
    }

    #[test]
    fn includes_provider_hint_for_grok() {
        let ctx = LlmReasonContext {
            session_id: None,
            agent_id: None,
            skill_name: None,
            provider_id: Some("grok"),
        };
        let out = build_reasoning_prompt("## User request\ntest", &ctx);
        assert!(out.contains("JSON array only"));
    }

    #[test]
    fn includes_session_id_hint() {
        let ctx = LlmReasonContext {
            session_id: Some("abc-123"),
            agent_id: None,
            skill_name: None,
            provider_id: None,
        };
        let out = build_reasoning_prompt("## User request\ntest", &ctx);
        assert!(out.contains("metadata.session_id = \"abc-123\""));
        assert!(out.contains("tool.execute"));
    }
}