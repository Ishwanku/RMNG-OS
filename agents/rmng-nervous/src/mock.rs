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

fn mem0_user_id() -> String {
    std::env::var("MEM0_DEFAULT_USER_ID").unwrap_or_else(|_| "rmng-os".into())
}

fn mem0_search_intent(query: &str, metadata: Option<Metadata>) -> CoreIntent {
    CoreIntent::McpProxy {
        mcp_server: "mem0".into(),
        mcp_tool: "search_memories".into(),
        mcp_args: serde_json::json!({
            "query": query,
            "user_id": mem0_user_id(),
            "limit": 5
        }),
        metadata,
    }
}


fn e2b_run_code_intent(code: &str, metadata: Option<Metadata>) -> CoreIntent {
    CoreIntent::McpProxy {
        mcp_server: "e2b".into(),
        mcp_tool: "run_code".into(),
        mcp_args: serde_json::json!({ "code": code }),
        metadata,
    }
}

fn extract_code_from_prompt(prompt: &str) -> String {
    if let Some(start) = prompt.find("```") {
        let rest = &prompt[start + 3..];
        let rest = rest
            .strip_prefix("python")
            .or_else(|| rest.strip_prefix("py"))
            .unwrap_or(rest);
        if let Some(end) = rest.find("```") {
            return rest[..end].trim().to_string();
        }
    }
    if let Some(idx) = prompt.find(':') {
        let tail = prompt[idx + 1..].trim();
        if !tail.is_empty() && tail.len() < 2000 {
            return tail.to_string();
        }
    }
    "print(2 + 2)".to_string()
}


fn default_test_harness() -> String {
    r#"errors = []
def check(name, cond):
    if not cond:
        errors.append(name)
check("smoke", 2 + 2 == 4)
if errors:
    print({"pass": False, "failed": errors})
else:
    print({"pass": True, "tests": 1})"#
    .to_string()
}

fn wants_run_tests(lower: &str) -> bool {
    lower.contains("run tests")
        || lower.contains("run-tests")
        || lower.contains("run test")
        || lower.contains("test harness")
        || lower.contains("pytest in sandbox")
}

fn wants_validate_test_output(lower: &str) -> bool {
    lower.contains("validate output")
        || lower.contains("validate-output")
        || lower.contains("validate test")
        || lower.contains("did the test pass")
        || lower.contains("check test results")
}

fn wants_test_coverage_check(lower: &str) -> bool {
    lower.contains("coverage check")
        || lower.contains("test-coverage")
        || lower.contains("test coverage")
        || lower.contains("coverage rubric")
}

fn wants_regression_check(lower: &str) -> bool {
    lower.contains("regression check")
        || lower.contains("regression-check")
        || lower.contains("regression test")
        || lower.contains("did we break")
}

fn testing_agents(agent_id: &str) -> bool {
    agent_id == "repo-keeper" || agent_id == "research-curator"
}

fn handle_testing_workflow(
    agent_id: &str,
    lower: &str,
    prompt: &str,
    session: Option<&AgentSession>,
    metadata: Option<Metadata>,
) -> Option<CoreIntent> {
    if !testing_agents(agent_id) {
        return None;
    }
    if wants_validate_test_output(lower)
        || wants_test_coverage_check(lower)
        || wants_regression_check(lower)
    {
        let summary = session
            .map(|s| s.tool_results_summary(5))
            .unwrap_or_else(|| "(no prior tool results)".to_string());
        let kind = if wants_regression_check(lower) {
            "regression-check"
        } else if wants_test_coverage_check(lower) {
            "test-coverage-check"
        } else {
            "validate-output"
        };
        return Some(CoreIntent::PlanOnly {
            reasoning: format!(
                "[{kind}] evaluate sandbox/test evidence.
Session context:
{summary}
User: {prompt}"
            ),
            metadata,
        });
    }
    if wants_run_tests(lower) || wants_sandbox_execution(lower) {
        let code = if wants_run_tests(lower) {
            default_test_harness()
        } else {
            extract_code_from_prompt(prompt)
        };
        return Some(e2b_run_code_intent(&code, metadata));
    }
    None
}


fn wants_sandbox_execution(lower: &str) -> bool {
    lower.contains("run code")
        || lower.contains("execute code")
        || lower.contains("sandbox")
        || lower.contains("run python")
        || lower.contains("verify script")
        || lower.contains("test code")
}


fn mem0_add_intent(text: &str, agent_id: &str, metadata: Option<Metadata>) -> CoreIntent {
    CoreIntent::McpProxy {
        mcp_server: "mem0".into(),
        mcp_tool: "add_memory".into(),
        mcp_args: serde_json::json!({
            "messages": [{ "role": "user", "content": text }],
            "user_id": mem0_user_id(),
            "agent_id": agent_id
        }),
        metadata,
    }
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
            if let Some(intent) =
                handle_testing_workflow(&a.id, &lower, prompt, session, metadata.clone())
            {
                return intent;
            }
            if lower.contains("delete memory") {
                return CoreIntent::McpProxy {
                    mcp_server: "mem0".into(),
                    mcp_tool: "delete_memory".into(),
                    mcp_args: serde_json::json!({
                        "memory_id": "placeholder-memory-id"
                    }),
                    metadata,
                };
            }
            if lower.contains("get memory") && lower.contains("memory_id") {
                return CoreIntent::McpProxy {
                    mcp_server: "mem0".into(),
                    mcp_tool: "get_memory".into(),
                    mcp_args: serde_json::json!({ "memory_id": "placeholder-memory-id" }),
                    metadata,
                };
            }
            if lower.contains("remember") || lower.contains("save memory") || lower.contains("add memory") {
                return mem0_add_intent(prompt, "research-curator", metadata);
            }
            if lower.contains("search memory") || lower.contains("recall") || lower.contains("prior memory") {
                let q = prompt
                    .split_whitespace()
                    .skip_while(|w| !w.contains("memory") && !w.eq_ignore_ascii_case("recall"))
                    .collect::<Vec<_>>()
                    .join(" ");
                let query = if q.is_empty() { prompt } else { q.as_str() };
                return mem0_search_intent(query, metadata);
            }
            if lower.contains("get issue") || lower.contains("issue #") {
                let number = prompt
                    .split_whitespace()
                    .find_map(|w| w.trim_start_matches('#').parse::<u64>().ok())
                    .unwrap_or(1);
                return CoreIntent::McpProxy {
                    mcp_server: "github".into(),
                    mcp_tool: "get_issue".into(),
                    mcp_args: serde_json::json!({
                        "owner": "Ishwanku",
                        "repo": "RMNG-OS",
                        "issue_number": number
                    }),
                    metadata,
                };
            }
            if lower.contains("list issues") || lower.contains("list open issues") {
                return CoreIntent::McpProxy {
                    mcp_server: "github".into(),
                    mcp_tool: "list_issues".into(),
                    mcp_args: serde_json::json!({
                        "owner": "Ishwanku",
                        "repo": "RMNG-OS",
                        "state": "open"
                    }),
                    metadata,
                };
            }
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
        if a.id == "web-researcher" {
            if lower.contains("remember") || lower.contains("save memory") || lower.contains("add memory") {
                return mem0_add_intent(prompt, "web-researcher", metadata);
            }
            if lower.contains("search memory") || lower.contains("recall") || lower.contains("prior memory") {
                return mem0_search_intent(prompt, metadata);
            }
            if lower.contains("pdf")
                || lower.contains("docx")
                || lower.contains("convert")
                || lower.contains("markitdown")
                || lower.contains("document")
                || lower.contains("file://")
            {
                let uri = if lower.contains("file://") {
                    prompt
                        .split_whitespace()
                        .find(|w| w.starts_with("file://"))
                        .unwrap_or("file:///tmp/sample.pdf")
                } else {
                    "https://example.com/sample.pdf"
                };
                return CoreIntent::McpProxy {
                    mcp_server: "markitdown".into(),
                    mcp_tool: "convert_to_markdown".into(),
                    mcp_args: serde_json::json!({ "uri": uri }),
                    metadata,
                };
            }
            if lower.contains("fetch")
                || lower.contains("http://")
                || lower.contains("https://")
                || lower.contains("url")
                || lower.contains("web page")
            {
                let url = prompt
                    .split_whitespace()
                    .find(|w| w.starts_with("http://") || w.starts_with("https://"))
                    .unwrap_or("https://example.com");
                return CoreIntent::McpProxy {
                    mcp_server: "fetch".into(),
                    mcp_tool: "fetch".into(),
                    mcp_args: serde_json::json!({
                        "url": url,
                        "max_length": 8000
                    }),
                    metadata,
                };
            }
            return CoreIntent::PlanOnly {
                reasoning: format!(
                    "Web research plan for: {prompt}. Use fetch for live URLs or markitdown for documents."
                ),
                metadata,
            };
        }
        if a.id == "browser-researcher" {
            if lower.contains("click") {
                return CoreIntent::McpProxy {
                    mcp_server: "playwright".into(),
                    mcp_tool: "browser_click".into(),
                    mcp_args: serde_json::json!({
                        "element": "Submit",
                        "ref": "e1"
                    }),
                    metadata,
                };
            }
            if lower.contains("snapshot") || lower.contains("a11y") || lower.contains("accessibility") {
                return CoreIntent::McpProxy {
                    mcp_server: "playwright".into(),
                    mcp_tool: "browser_snapshot".into(),
                    mcp_args: serde_json::json!({}),
                    metadata,
                };
            }
            if lower.contains("navigate")
                || lower.contains("browser")
                || lower.contains("http://")
                || lower.contains("https://")
            {
                let url = prompt
                    .split_whitespace()
                    .find(|w| w.starts_with("http://") || w.starts_with("https://"))
                    .unwrap_or("https://example.com");
                return CoreIntent::McpProxy {
                    mcp_server: "playwright".into(),
                    mcp_tool: "browser_navigate".into(),
                    mcp_args: serde_json::json!({ "url": url }),
                    metadata,
                };
            }
            return CoreIntent::PlanOnly {
                reasoning: format!(
                    "Browser research plan for: {prompt}. Requires playwright MCP (opt-in)."
                ),
                metadata,
            };
        }
        if a.id == "repo-keeper" {
            if let Some(intent) =
                handle_testing_workflow(&a.id, &lower, prompt, session, metadata.clone())
            {
                return intent;
            }
            if lower.contains("search memory") || lower.contains("recall memory") {
                return mem0_search_intent(prompt, metadata);
            }
            if lower.contains("get memory") {
                return CoreIntent::McpProxy {
                    mcp_server: "mem0".into(),
                    mcp_tool: "get_memory".into(),
                    mcp_args: serde_json::json!({ "memory_id": "placeholder-memory-id" }),
                    metadata,
                };
            }
            let repo = std::env::var("HOME")
                .map(|h| format!("{h}/dev/projects/RMNG-OS"))
                .unwrap_or_else(|_| ".".into());
            if lower.contains("mcp") && (lower.contains("diff") || lower.contains("changes")) {
                return CoreIntent::McpProxy {
                    mcp_server: "git".into(),
                    mcp_tool: "git.diff".into(),
                    mcp_args: serde_json::json!({
                        "repo_path": repo,
                        "staged": false
                    }),
                    metadata,
                };
            }
            if lower.contains("mcp") && lower.contains("status") {
                return CoreIntent::McpProxy {
                    mcp_server: "git".into(),
                    mcp_tool: "git.status".into(),
                    mcp_args: serde_json::json!({ "repo_path": repo }),
                    metadata,
                };
            }
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