use super::types::LlmReasonContext;

const INTENT_EXAMPLES: &str = r#"
Example intents (emit exactly one JSON object):
{"action":"tool.execute","target":"git.status","parameters":{},"metadata":{"session_id":"<sid>"}}
{"action":"mcp.proxy","mcp_server":"github","mcp_tool":"search_issues","mcp_args":{"query":"repo:Ishwanku/RMNG-OS is:open"},"metadata":{"session_id":"<sid>"}}
{"action":"plan.only","reasoning":"Task complete. Summarize prior tool results.","metadata":{"session_id":"<sid>"}}
{"action":"plan.only","reasoning":"Delegate via chain.","metadata":{"session_id":"<sid>","handoff_chain":["swarm-coordinator","repo-keeper","runtime-executor"]}}
{"action":"plan.only","reasoning":"Delegate to specialist.","metadata":{"session_id":"<sid>","handoff_to":"repo-keeper"}}
{"action":"plan.only","reasoning":"Specialist done; return summary.","metadata":{"session_id":"<sid>","handoff_return_to":"swarm-coordinator"}}
"#;

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
    format!(
        "{assembled}{hint_block}{INTENT_EXAMPLES}\nRespond with a single JSON object for core-intent v2. No markdown, no prose outside JSON."
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn includes_session_id_hint() {
        let ctx = LlmReasonContext {
            session_id: Some("abc-123"),
            agent_id: None,
            skill_name: None,
        };
        let out = build_reasoning_prompt("## User request\ntest", &ctx);
        assert!(out.contains("metadata.session_id = \"abc-123\""));
        assert!(out.contains("tool.execute"));
    }
}