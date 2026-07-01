//! LLM-facing orchestration guidance for multi-hop chains (Sprint 24).

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
"#;

pub fn agent_registry_block() -> String {
    format!(
        "## Valid agent ids for handoff metadata\n{}\n",
        KNOWN_AGENT_IDS.join(", ")
    )
}

/// Extra hints injected when a session is active (layer-aware).
pub fn session_chain_hints(session: &AgentSession, agent: Option<&AgentDefinition>) -> String {
    let mut parts = vec![agent_registry_block(), CHAIN_RECIPES.to_string()];

    if let Some(orch) = session.shared_context.get("orchestration") {
        parts.push(format!(
            "## Active orchestration (do not restart chain)\n{orch}\n\
             If status is in_progress: execute your tools OR return with handoff_return_to.\n\
             If status is completed: synthesize with plan.only or handoff_return_to if you are the last specialist."
        ));
    }

    if let Some(a) = agent {
        match a.layer {
            AgentLayer::L4 => {
                parts.push(
                    "## L4 orchestrator\n\
                     Prefer `handoff_chain` for multi-step workflows. \
                     Use `handoff_to` only for a single specialist. \
                     Never emit tool.execute yourself — delegate."
                        .into(),
                );
            }
            AgentLayer::L3 | AgentLayer::L2 => {
                parts.push(format!(
                    "## {} specialist\n\
                     After successful tool results, emit plan.only with \
                     metadata.handoff_return_to = \"swarm-coordinator\" unless more work is required.",
                    a.id
                ));
            }
            _ => {}
        }
    }

    parts.join("\n")
}
