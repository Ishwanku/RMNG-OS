use serde_yaml::Value;
use std::path::{Path, PathBuf};

const BASE_SYSTEM_INSTRUCTIONS: &str = r#"You are the RMNG-OS nervous system (BYO-LLM reasoning plane).
Output ONLY valid JSON matching the v2 core-intent schema with top-level "action":
- tool.execute — native tools via rmngd (target + parameters)
- mcp.proxy — allowlisted MCP tools (mcp_server + mcp_tool + mcp_args)
- plan.only — reasoning only, no execution

Never output shell commands. Never execute tools directly."#;

const SESSION_ORCHESTRATION_GUIDE: &str = r#"## Multi-agent session orchestration

You are operating inside a persistent RMNG session. The Session context block contains:
- `recent_tool_results` — outputs from prior rmngd executions (read these first)
- `recent_handoffs` — which agents already worked on this task
- `shared_context` — operator-provided keys and values

Decision rules:
1. **Continue** — if more work is needed within your allowed tools, emit `tool.execute` or `mcp.proxy`.
2. **Complete** — if prior results satisfy the request, emit `plan.only` with a concise summary.
3. **Delegate** — set `metadata.handoff_to` to the target agent id (e.g. `repo-keeper`, `kernel-engineer`) and emit `plan.only` with a brief delegation reason. The router auto-handoffs when a session is active.
4. **Delegate via tool (L4 fallback)** — emit `tool.execute` for the target tool; the router hands off to the right L3/L2 agent.

Always include `metadata.session_id` matching the session when a session is active.
Set `metadata.handoff_to` only when another agent should take over (must be a valid agent id from the registry).
Never repeat a tool call if `recent_tool_results` already contains a successful result for the same tool unless the user explicitly asks to re-run."#;

/// Lightweight skill entry for progressive disclosure (index only).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillSummary {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct AgentSkill {
    pub metadata: Value,
    pub instructions: String,
}

#[derive(Debug, thiserror::Error)]
pub enum SkillError {
    #[error("skill not found: {0}")]
    NotFound(String),
    #[error("read skill file: {0}")]
    Io(#[from] std::io::Error),
    #[error("parse skill frontmatter: {0}")]
    Parse(String),
}

/// Resolve `skills/` directory (RMNG-OS repo root).
pub fn skills_root() -> PathBuf {
    if let Ok(root) = std::env::var("RMNG_PROJECT_ROOT") {
        let path = PathBuf::from(root).join("skills");
        if path.is_dir() {
            return path;
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        let path = PathBuf::from(home).join("dev/projects/RMNG-OS/skills");
        if path.is_dir() {
            return path;
        }
    }
    PathBuf::from("skills")
}

/// Load index: frontmatter `name` + `description` only (progressive disclosure stage 1).
pub fn load_skill_index() -> Result<Vec<SkillSummary>, SkillError> {
    let root = skills_root();
    let mut summaries = Vec::new();
    if !root.is_dir() {
        return Ok(summaries);
    }
    for entry in std::fs::read_dir(&root)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let skill_md = path.join("SKILL.md");
        if !skill_md.is_file() {
            continue;
        }
        let raw = std::fs::read_to_string(&skill_md)?;
        let (metadata, _) = parse_frontmatter(&raw, &skill_md)?;
        let name = metadata
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| path.file_name().unwrap().to_str().unwrap_or("unknown"))
            .to_string();
        let description = metadata
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        summaries.push(SkillSummary { name, description });
    }
    summaries.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(summaries)
}

/// Load full `skills/<skill_name>/SKILL.md` (progressive disclosure stage 2).
pub fn load_skill(skill_name: &str) -> Result<AgentSkill, SkillError> {
    let path = skills_root().join(skill_name).join("SKILL.md");
    if !path.is_file() {
        return Err(SkillError::NotFound(format!(
            "no SKILL.md at {}",
            path.display()
        )));
    }
    let raw = std::fs::read_to_string(&path)?;
    parse_skill_md(&raw, &path)
}

/// Load all skills declared on an agent definition (activation time).
pub fn load_skills_for_agent(
    agent: &crate::agent::AgentDefinition,
) -> Result<Vec<AgentSkill>, SkillError> {
    let mut skills = Vec::new();
    for name in &agent.skills {
        skills.push(load_skill(name)?);
    }
    Ok(skills)
}

fn parse_frontmatter(content: &str, path: &Path) -> Result<(Value, usize), SkillError> {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return Err(SkillError::Parse(format!(
            "missing YAML frontmatter in {}",
            path.display()
        )));
    }
    let after_first = trimmed.strip_prefix("---").unwrap_or(trimmed);
    let end = after_first.find("\n---").ok_or_else(|| {
        SkillError::Parse(format!("unclosed frontmatter in {}", path.display()))
    })?;
    let yaml_part = after_first[..end].trim();
    let body_start = end + 4;
    let metadata: Value = serde_yaml::from_str(yaml_part)
        .map_err(|e| SkillError::Parse(format!("{}: {e}", path.display())))?;
    Ok((metadata, body_start))
}

fn parse_skill_md(content: &str, path: &Path) -> Result<AgentSkill, SkillError> {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return Err(SkillError::Parse(format!(
            "missing YAML frontmatter in {}",
            path.display()
        )));
    }
    let after_first = trimmed.strip_prefix("---").unwrap_or(trimmed);
    let end = after_first.find("\n---").ok_or_else(|| {
        SkillError::Parse(format!("unclosed frontmatter in {}", path.display()))
    })?;
    let yaml_part = after_first[..end].trim();
    let instructions = after_first[end + 4..].trim().to_string();
    let metadata: Value = serde_yaml::from_str(yaml_part)
        .map_err(|e| SkillError::Parse(format!("{}: {e}", path.display())))?;
    Ok(AgentSkill {
        metadata,
        instructions,
    })
}

/// Assemble nervous-system context: base → agent → skills → user prompt.
pub fn assemble_prompt(skill: Option<&AgentSkill>, user_prompt: &str) -> String {
    assemble_prompt_with_agent(None, &[], skill, user_prompt)
}

pub fn assemble_prompt_with_agent(
    agent: Option<&crate::agent::AgentDefinition>,
    extra_skills: &[AgentSkill],
    primary_skill: Option<&AgentSkill>,
    user_prompt: &str,
) -> String {
    assemble_prompt_full(agent, extra_skills, primary_skill, None, user_prompt)
}

pub fn assemble_prompt_full(
    agent: Option<&crate::agent::AgentDefinition>,
    extra_skills: &[AgentSkill],
    primary_skill: Option<&AgentSkill>,
    session: Option<&rmng_core::AgentSession>,
    user_prompt: &str,
) -> String {
    let mut parts = vec![BASE_SYSTEM_INSTRUCTIONS.to_string()];

    if let Some(a) = agent {
        parts.push(format!(
            "## Agent: {}\n{}\n\nAllowed native tools: {}\nAllowed MCP tools: {}",
            a.id,
            a.description,
            if a.allowed_native_tools.is_empty() {
                "(none)".into()
            } else {
                a.allowed_native_tools.join(", ")
            },
            if a.allowed_mcp_tools.is_empty() {
                "(none)".into()
            } else {
                a.allowed_mcp_tools.join(", ")
            }
        ));
    }

    if let Some(sess) = session {
        parts.push(SESSION_ORCHESTRATION_GUIDE.to_string());
        let ctx = sess.prompt_context();
        if !ctx.is_empty() {
            parts.push(format!("## Session context\n{ctx}"));
        }
    }

    let primary_name = primary_skill
        .and_then(|s| s.metadata.get("name"))
        .and_then(|v| v.as_str());

    if let Some(s) = primary_skill {
        parts.push(format!("## Skill instructions\n{}", s.instructions));
    }

    for s in extra_skills {
        let name = s
            .metadata
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("skill");
        if primary_name == Some(name) {
            continue;
        }
        parts.push(format!("## Additional skill: {name}\n{}", s.instructions));
    }

    parts.push(format!("## User request\n{user_prompt}"));
    parts.join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_frontmatter_and_body() {
        let raw = r#"---
name: test-skill
description: A test
---

# Heading

Do the thing.
"#;
        let skill = parse_skill_md(raw, Path::new("test.md")).unwrap();
        assert_eq!(skill.metadata["name"], "test-skill");
        assert!(skill.instructions.contains("# Heading"));
    }

    #[test]
    fn loads_skill_index_without_full_body() {
        let index = load_skill_index();
        if index.is_err() {
            return;
        }
        let index = index.unwrap();
        if index.is_empty() {
            return;
        }
        assert!(index[0].description.len() > 0 || index[0].name.len() > 0);
    }

    #[test]
    fn session_context_in_prompt() {
        use rmng_core::SessionStore;
        let dir = std::env::temp_dir().join(format!("rmng-skill-ctx-{}", uuid::Uuid::new_v4()));
        let store = SessionStore::new(&dir);
        let mut session = store.create().unwrap();
        store
            .set_context(&mut session, "note", serde_json::json!("hello"))
            .unwrap();
        let prompt = assemble_prompt_full(None, &[], None, Some(&session), "test");
        assert!(prompt.contains("hello"));
        assert!(prompt.contains("Session context"));
        assert!(prompt.contains("Multi-agent session orchestration"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn loads_git_workflow_from_repo() {
        let skill = load_skill("git-workflow");
        if skill.is_err() {
            return;
        }
        let skill = skill.unwrap();
        assert_eq!(skill.metadata["name"], "git-workflow");
        assert!(skill.instructions.contains("git.status"));
    }
}