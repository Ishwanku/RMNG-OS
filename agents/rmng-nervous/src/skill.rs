use serde_yaml::Value;
use std::path::{Path, PathBuf};

const BASE_SYSTEM_INSTRUCTIONS: &str = r#"You are the RMNG-OS nervous system (BYO-LLM reasoning plane).
Output ONLY valid JSON matching the v2 core-intent schema with top-level "action":
- tool.execute — native tools via rmngd (target + parameters)
- mcp.proxy — allowlisted MCP tools (mcp_server + mcp_tool + mcp_args)
- plan.only — reasoning only, no execution

Never output shell commands. Never execute tools directly."#;

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

/// Load `skills/<skill_name>/SKILL.md` and parse YAML frontmatter + markdown body.
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

/// Assemble nervous-system context: base → skill → user prompt.
pub fn assemble_prompt(skill: Option<&AgentSkill>, user_prompt: &str) -> String {
    let mut parts = vec![BASE_SYSTEM_INSTRUCTIONS.to_string()];
    if let Some(s) = skill {
        parts.push(format!("## Skill instructions\n{}", s.instructions));
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
    fn loads_git_workflow_from_repo() {
        let skill = load_skill("git-workflow");
        if skill.is_err() {
            // OK in CI without full repo layout
            return;
        }
        let skill = skill.unwrap();
        assert_eq!(skill.metadata["name"], "git-workflow");
        assert!(skill.instructions.contains("git.status"));
    }
}