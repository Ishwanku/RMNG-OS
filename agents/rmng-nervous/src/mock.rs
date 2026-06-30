use rmng_core::{CoreIntent, Metadata};

fn skill_metadata(skill_name: Option<&str>) -> Option<Metadata> {
    skill_name.map(|name| Metadata {
        skill_name: Some(name.to_string()),
        trace_id: None,
    })
}

/// Nervous-system stub when `LlmProvider::None` — returns valid v2 `CoreIntent` JSON shapes.
pub fn mock_core_intent(
    prompt: &str,
    skill_name: Option<&str>,
    skill_instructions: Option<&str>,
) -> CoreIntent {
    let metadata = skill_metadata(skill_name);
    let lower = prompt.to_lowercase();
    let skill_lower = skill_instructions.unwrap_or("").to_lowercase();

    // git-workflow skill or git-related prompt
    if skill_name == Some("git-workflow")
        || lower.contains("git")
        || skill_lower.contains("git.status")
    {
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

    #[test]
    fn mock_git_status_with_skill() {
        let intent = mock_core_intent("check git status", Some("git-workflow"), None);
        assert!(matches!(intent, CoreIntent::ToolExecute { .. }));
        assert_eq!(
            intent.metadata().and_then(|m| m.skill_name.as_deref()),
            Some("git-workflow")
        );
    }

    #[test]
    fn mock_plan_only_default() {
        let intent = mock_core_intent("hello world", None, None);
        assert!(matches!(intent, CoreIntent::PlanOnly { .. }));
    }
}