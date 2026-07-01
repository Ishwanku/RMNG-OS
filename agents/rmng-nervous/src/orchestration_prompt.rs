//! LLM-facing orchestration guidance for multi-hop chains (Sprint 24–30).

use crate::agent::AgentDefinition;
use crate::layer::AgentLayer;
use rmng_core::AgentSession;

/// Canonical agent ids valid in handoff metadata (loaded from definitions/).
pub const KNOWN_AGENT_IDS: &[&str] = &[
    "swarm-coordinator",
    "research-curator",
    "repo-keeper",
    "kernel-engineer",
    "system-health",
    "runtime-executor",
    "web-researcher",
    "browser-researcher",
];

pub const CHAIN_RECIPES: &str = r#"## Chain recipes (use metadata.handoff_chain)

| Task pattern | Suggested chain | Notes |
|--------------|-----------------|-------|
| Git hygiene → run check | `["swarm-coordinator","repo-keeper","runtime-executor"]` | L4 plans, L3 reads repo, L2 runs if needed |
| Research → repo change | `["swarm-coordinator","research-curator","repo-keeper"]` | Curator gathers; repo-keeper executes git tools |
| Specialist done → synthesize | `plan.only` + `handoff_return_to: "swarm-coordinator"` | L3/L2 only; include summary from recent_tool_results |

Rules:
- `handoff_chain` MUST be JSON array of agent id strings (min 2 hops).
- First id MUST be your current agent id when you initiate the chain.
- Do NOT restart a chain if `orchestration_chain.status` is `in_progress` — continue or return instead.
- Anti-patterns (break parse): comma-string handoff_chain, arrow strings, markdown fences, `plan_only` action (use `plan.only`).
- After executing tools successfully, specialists SHOULD `handoff_return_to` the orchestrator unless the user asked for more work.
- Grok/Groq: NEVER write `"handoff_chain":"a,b"` — ONLY `"handoff_chain":["a","b"]`.
"#;

/// Copy-paste few-shot examples for live models (Sprint 30).
pub const FEW_SHOT_CHAIN_EXAMPLES: &str = r#"## Few-shot: correct chain emission (copy structure exactly)

Git workflow (L4 starts chain):
{"action":"plan.only","reasoning":"Delegate git hygiene: coordinator plans, repo-keeper inspects, executor runs checks.","metadata":{"session_id":"<sid>","chain_id":"<sid>","handoff_chain":["swarm-coordinator","repo-keeper","runtime-executor"],"hop_failure_policy":"skip"}}

Research workflow:
{"action":"plan.only","reasoning":"Research then repo update.","metadata":{"session_id":"<sid>","handoff_chain":["swarm-coordinator","research-curator","repo-keeper"]}}

Specialist returns to orchestrator (L3/L2 after tools succeed):
{"action":"plan.only","reasoning":"Git status captured; summarizing for orchestrator.","metadata":{"session_id":"<sid>","handoff_return_to":"swarm-coordinator"}}

Single specialist (NOT a chain — use handoff_to only):
{"action":"plan.only","reasoning":"Need repo read only.","metadata":{"session_id":"<sid>","handoff_to":"repo-keeper"}}
"#;

/// Guidance when prior hop failed, circuit open, or budget warned (Sprint 30).
pub const ERROR_RECOVERY_HINTS: &str = r#"## Error recovery (circuit breaker, budget, failed hop)

If Session context shows `orchestration.status` = `failed` or a hop was skipped:
- Do NOT re-emit the same `handoff_chain` from scratch.
- Emit `plan.only` with `handoff_return_to: "swarm-coordinator"` and summarize what succeeded/failed.
- Or emit a shorter chain starting from YOUR agent id with `hop_failure_policy: "skip"`.

If you cannot call LLM tools (budget deny / circuit open in recent audit):
- Emit `plan.only` explaining the block — do NOT emit tool.execute.
- Prefer `handoff_return_to` so the orchestrator can synthesize or retry later.

Never emit shell commands or free-text plans outside the JSON intent.
"#;

pub fn agent_registry_block() -> String {
    format!(
        "## Valid agent ids for handoff metadata\n{}\n",
        KNOWN_AGENT_IDS.join(", ")
    )
}

/// Extra hints injected when a session is active (layer-aware).
pub fn session_chain_hints(session: &AgentSession, agent: Option<&AgentDefinition>) -> String {
    let mut parts = vec![
        agent_registry_block(),
        CHAIN_RECIPES.to_string(),
        FEW_SHOT_CHAIN_EXAMPLES.to_string(),
        ERROR_RECOVERY_HINTS.to_string(),
    ];

    if let Some(orch) = session.shared_context.get("orchestration") {
        parts.push(format!(
            "## Active orchestration (do not restart chain)\n{orch}\n\
             If status is in_progress: execute your tools OR return with handoff_return_to.\n\
             If status is failed: return to orchestrator with summary — do not restart full chain.\n\
             If status is completed: synthesize with plan.only or handoff_return_to if you are the last specialist."
        ));
    }

    if let Some(a) = agent {
        match a.layer {
            AgentLayer::L4 => {
                parts.push(
                    "## L4 orchestrator\n\
                     For multi-step workflows ALWAYS use `handoff_chain` (JSON array, min 2 ids).\n\
                     Use `handoff_to` only when exactly one specialist is needed.\n\
                     Never emit tool.execute yourself — delegate.\n\
                     Example first hop: [\"swarm-coordinator\",\"repo-keeper\",\"runtime-executor\"]"
                        .into(),
                );
            }
            AgentLayer::L3 | AgentLayer::L2 => {
                parts.push(format!(
                    "## {} specialist\n\
                     After successful tool results, emit plan.only with \
                     metadata.handoff_return_to = \"swarm-coordinator\" and cite recent_tool_results.\n\
                     Do NOT emit handoff_chain unless you are L4.",
                    a.id
                ));
            }
            _ => {}
        }
    }

    parts.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmng_core::SessionStore;

    #[test]
    fn chain_hints_include_few_shot_and_recovery() {
        let dir = std::env::temp_dir().join(format!("rmng-orch-prompt-{}", uuid::Uuid::new_v4()));
        let store = SessionStore::new(&dir);
        let session = store.create().expect("session");
        let out = session_chain_hints(&session, None);
        assert!(out.contains("handoff_chain"));
        assert!(out.contains("handoff_return_to"));
        assert!(out.contains("circuit breaker"));
        assert!(out.contains("swarm-coordinator\",\"repo-keeper"));
        let _ = std::fs::remove_dir_all(dir);
    }
}