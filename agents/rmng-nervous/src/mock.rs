use crate::agent::AgentDefinition;
use rmng_core::{AgentSession, CoreIntent, Metadata};

fn skill_metadata(skill_name: Option<&str>) -> Option<Metadata> {
    skill_name.map(|name| Metadata {
        skill_name: Some(name.to_string()),
        session_id: None,
        handoff_from: None,
        handoff_to: None,
        handoff_chain: None,
        trace_id: None,
    })
}

fn session_metadata(
    skill_name: Option<&str>,
    session: Option<&AgentSession>,
) -> Option<Metadata> {
    let mut meta = skill_metadata(skill_name).unwrap_or(Metadata {
        skill_name: None,
        session_id: None,
        handoff_from: None,
        handoff_to: None,
        handoff_chain: None,
        trace_id: None,
    });
    if let Some(sess) = session {
        meta.session_id = Some(sess.id.clone());
        meta.trace_id = Some(sess.id.clone());
    }
    Some(meta)
}

/// Nervous-system stub when `LlmProvider::None` — returns valid v2 `CoreIntent` JSON shapes.
pub fn mock_core_intent(
    prompt: &str,
    skill_name: Option<&str>,
    skill_instructions: Option<&str>,
    agent: Option<&AgentDefinition>,
    session: Option<&AgentSession>,
) -> CoreIntent {
    let metadata = session_metadata(skill_name, session);
    let lower = prompt.to_lowercase();
    let skill_lower = skill_instructions.unwrap_or("").to_lowercase();

    // Session-aware: summarize prior tool results instead of re-executing
    if let Some(sess) = session {
        let summary = sess.tool_results_summary(3);
        if summary != "(no prior tool results)"
            && (lower.contains("summarize")
                || lower.contains("previous")
                || lower.contains("prior")
                || lower.contains("complete")
                || lower.contains("done"))
        {
            return CoreIntent::PlanOnly {
                reasoning: format!(
                    "Session {id}: synthesizing prior results.\n{summary}\nUser request: {prompt}",
                    id = sess.id
                ),
                metadata,
            };
        }
    }

    // Agent-scoped routing hints
    if let Some(a) = agent {
        if a.id == "swarm-coordinator" {
            if lower.contains("git") || lower.contains("repo") || lower.contains("status") {
                return CoreIntent::ToolExecute {
                    target: "git.status".into(),
                    parameters: serde_json::json!({}),
                    metadata,
                };
            }
            if lower.contains("kernel") || lower.contains("health") || lower.contains("build") {
                return CoreIntent::ToolExecute {
                    target: "kernel.status".into(),
                    parameters: serde_json::json!({}),
                    metadata,
                };
            }
            return CoreIntent::PlanOnly {
                reasoning: format!(
                    "Orchestrator plan for: {prompt}. Delegate to specialists via handoff."
                ),
                metadata,
            };
        }
        if a.id == "system-health" {
            return CoreIntent::ToolExecute {
                target: "kernel.status".into(),
                parameters: serde_json::json!({}),
                metadata,
            };
        }
        if a.id == "research-curator" {
            if lower.contains("search") || lower.contains("issue") {
                return CoreIntent::McpProxy {
                    mcp_server: "github".into(),
                    mcp_tool: "search_issues".into(),
                    mcp_args: serde_json::json!({ "query": "repo:Ishwanku/RMNG-OS is:open" }),
                    metadata,
                };
            }
            return CoreIntent::PlanOnly {
                reasoning: format!("Research summary for: {prompt}"),
                metadata,
            };
        }
        if a.id == "repo-keeper" {
            if lower.contains("diff") || lower.contains("changes") {
                return CoreIntent::ToolExecute {
                    target: "git.diff".into(),
                    parameters: serde_json::json!({}),
                    metadata,
                };
            }
            if lower.contains("pr") || lower.contains("pull request") {
                return CoreIntent::ToolExecute {
                    target: "github.pr_status".into(),
                    parameters: serde_json::json!({}),
                    metadata,
                };
            }
            if lower.contains("log") || lower.contains("history") || lower.contains("commits") {
                let repo = std::env::var("HOME")
                    .map(|h| format!("{h}/dev/projects/RMNG-OS"))
                    .unwrap_or_else(|_| ".".into());
                return CoreIntent::McpProxy {
                    mcp_server: "git".into(),
                    mcp_tool: "git.log".into(),
                    mcp_args: serde_json::json!({
                        "repo_path": repo,
                        "max_count": 3
                    }),
                    metadata,
                };
            }
            if lower.contains("status") || lower.contains("check") {
                return CoreIntent::ToolExecute {
                    target: "git.status".into(),
                    parameters: serde_json::json!({}),
                    metadata,
                };
            }
        }
        if a.id == "kernel-engineer" {
            if lower.contains("build") && !lower.contains("status") {
                return CoreIntent::ToolExecute {
                    target: "kernel.build".into(),
                    parameters: serde_json::json!({ "target": "all" }),
                    metadata,
                };
            }
            return CoreIntent::ToolExecute {
                target: "kernel.status".into(),
                parameters: serde_json::json!({}),
                metadata,
            };
        }
    }

    // git-workflow skill or git-related prompt
    if skill_name == Some("git-workflow")
        || lower.contains("git")
        || skill_lower.contains("git.status")
    {
        if lower.contains("diff") {
            return CoreIntent::ToolExecute {
                target: "git.diff".into(),
                parameters: serde_json::json!({}),
                metadata,
            };
        }
        if lower.contains("log") || lower.contains("history") || lower.contains("commits") {
            let repo = std::env::var("HOME")
                .map(|h| format!("{h}/dev/projects/RMNG-OS"))
                .unwrap_or_else(|_| ".".into());
            return CoreIntent::McpProxy {
                mcp_server: "git".into(),
                mcp_tool: "git.log".into(),
                mcp_args: serde_json::json!({
                    "repo_path": repo,
                    "max_count": 3
                }),
                metadata,
            };
        }
        if lower.contains("status") || lower.contains("check") || lower.contains("hygiene") {
            return CoreIntent::ToolExecute {
                target: "git.status".into(),
                parameters: serde_json::json!({}),
                metadata,
            };
        }
    }

    if skill_name == Some("github-workflow") || lower.contains("pull request") || lower.contains("pr status") {
        return CoreIntent::ToolExecute {
            target: "github.pr_status".into(),
            parameters: serde_json::json!({}),
            metadata,
        };
    }

    if skill_name == Some("kernel-build")
        || skill_name == Some("kernel-config")
        || lower.contains("kernel")
        || lower.contains("build")
    {
        if lower.contains("build") && !lower.contains("status") {
            return CoreIntent::ToolExecute {
                target: "kernel.build".into(),
                parameters: serde_json::json!({ "target": "all" }),
                metadata,
            };
        }
        return CoreIntent::ToolExecute {
            target: "kernel.status".into(),
            parameters: serde_json::json!({}),
            metadata,
        };
    }

    if skill_name == Some("phase-gates") {
        return CoreIntent::PlanOnly {
            reasoning: format!(
                "Review docs/ROADMAP.md success criteria before marking complete. Task: {prompt}"
            ),
            metadata,
        };
    }

    CoreIntent::PlanOnly {
        reasoning: format!(
            "[mock nervous-system] no LLM provider configured — received: {prompt}"
        ),
        metadata,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::AgentRegistry;
    use std::path::PathBuf;

    #[test]
    fn mock_git_status_with_skill() {
        let intent = mock_core_intent("check git status", Some("git-workflow"), None, None, None);
        assert!(matches!(intent, CoreIntent::ToolExecute { .. }));
    }

    #[test]
    fn mock_repo_keeper_agent_status() {
        let reg = AgentRegistry::load_from(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../definitions"),
        )
        .unwrap();
        let agent = reg.get("repo-keeper").unwrap();
        let intent =
            mock_core_intent("check git status", Some("git-workflow"), None, Some(agent), None);
        match intent {
            CoreIntent::ToolExecute { target, .. } => assert_eq!(target, "git.status"),
            _ => panic!("expected tool.execute"),
        }
    }

    #[test]
    fn mock_plan_only_default() {
        let intent = mock_core_intent("hello world", None, None, None, None);
        assert!(matches!(intent, CoreIntent::PlanOnly { .. }));
    }

    #[test]
    fn mock_session_summarizes_prior_tool_results() {
        use rmng_core::{SessionStore, ToolResultRecord};
        let dir = std::env::temp_dir().join(format!("rmng-mock-ctx-{}", uuid::Uuid::new_v4()));
        let store = SessionStore::new(&dir);
        let mut session = store.create().unwrap();
        store
            .record_tool_result(
                &mut session,
                ToolResultRecord {
                    timestamp: chrono::Utc::now(),
                    tool: "git.status".into(),
                    parameters: serde_json::json!({}),
                    output: "clean".into(),
                    success: true,
                    exit_code: Some(0),
                    handoff_from: None,
                },
            )
            .unwrap();
        let intent = mock_core_intent(
            "summarize previous results",
            None,
            None,
            None,
            Some(&session),
        );
        match intent {
            CoreIntent::PlanOnly { reasoning, .. } => assert!(reasoning.contains("git.status")),
            _ => panic!("expected plan.only"),
        }
        let _ = std::fs::remove_dir_all(dir);
    }
}