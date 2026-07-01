use crate::allowlist::{McpAllowlist, McpServerConfig};
use crate::intent::{CoreIntent, Intent, IntentKind};
use crate::registry::IntegrationRegistry;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionVerdict {
    Allow,
    Deny(String),
}

#[derive(Clone)]
pub struct PermissionGate {
    allowed_tools: Vec<String>,
    mcp_allowlist: McpAllowlist,
}

impl Default for PermissionGate {
    fn default() -> Self {
        match IntegrationRegistry::load() {
            Ok(reg) => Self::from_registry(&reg),
            Err(e) => {
                tracing::warn!(error = %e, "integration registry load failed — empty native allowlist");
                Self::from_registry(
                    &IntegrationRegistry::load_from(std::path::Path::new("/nonexistent")).unwrap(),
                )
            }
        }
    }
}

impl PermissionGate {
    /// Build gate with native tools derived from integration manifests.
    pub fn from_registry(registry: &IntegrationRegistry) -> Self {
        Self {
            allowed_tools: registry.allowed_tool_names(),
            mcp_allowlist: McpAllowlist::load().unwrap_or_default(),
        }
    }

    pub fn new(allowed_tools: Vec<String>) -> Self {
        Self {
            allowed_tools,
            mcp_allowlist: McpAllowlist::load().unwrap_or_default(),
        }
    }

    pub fn allowed_tools(&self) -> &[String] {
        &self.allowed_tools
    }

    pub fn with_mcp_allowlist(mut self, allowlist: McpAllowlist) -> Self {
        self.mcp_allowlist = allowlist;
        self
    }

    pub fn mcp_allowlist(&self) -> &McpAllowlist {
        &self.mcp_allowlist
    }

    pub fn mcp_server_config(&self, server: &str) -> Option<&McpServerConfig> {
        self.mcp_allowlist.server_config(server)
    }

    pub fn evaluate(&self, intent: &Intent) -> PermissionVerdict {
        match intent.kind {
            IntentKind::Plan | IntentKind::Clarify | IntentKind::Complete => PermissionVerdict::Allow,
            IntentKind::ToolRequest => {
                let Some(tool) = &intent.tool else {
                    return PermissionVerdict::Deny("tool_request missing tool field".into());
                };
                self.evaluate_tool_name(&tool.name)
            }
        }
    }

    pub fn evaluate_core(&self, intent: &CoreIntent) -> PermissionVerdict {
        match intent {
            CoreIntent::PlanOnly { .. } => PermissionVerdict::Allow,
            CoreIntent::ToolExecute { target, .. } => self.evaluate_tool_name(target),
            CoreIntent::McpProxy {
                mcp_server,
                mcp_tool,
                ..
            } => self.evaluate_mcp(mcp_server, mcp_tool),
        }
    }

    fn evaluate_tool_name(&self, name: &str) -> PermissionVerdict {
        if self.allowed_tools.iter().any(|t| t == name) {
            PermissionVerdict::Allow
        } else {
            PermissionVerdict::Deny(format!("tool not allowed: {name}"))
        }
    }

    fn evaluate_mcp(&self, server: &str, tool: &str) -> PermissionVerdict {
        let Some(cfg) = self.mcp_allowlist.servers.get(server) else {
            return PermissionVerdict::Deny(format!("mcp server not configured: {server}"));
        };
        if !cfg.enabled {
            return PermissionVerdict::Deny(format!("mcp server disabled: {server}"));
        }
        if !cfg.allowed_tools.iter().any(|t| t == tool) {
            return PermissionVerdict::Deny(format!("mcp tool not allowed: {server}.{tool}"));
        }
        PermissionVerdict::Allow
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intent::{CoreIntent, Intent, IntentKind};
    use crate::registry::IntegrationRegistry;
    use std::collections::HashMap;
    use uuid::Uuid;

    fn fixture_registry() -> IntegrationRegistry {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../integrations");
        IntegrationRegistry::load_from(root).expect("fixture integrations")
    }

    fn test_gate_with_mcp() -> PermissionGate {
        let mut servers = HashMap::new();
        servers.insert(
            "git".into(),
            McpServerConfig {
                enabled: true,
                command: "uvx".into(),
                args: vec!["mcp-server-git".into()],
                allowed_tools: vec!["git.log".into(), "git.diff".into(), "git.status".into()],
                isolation: None,
            },
        );
        PermissionGate::from_registry(&fixture_registry()).with_mcp_allowlist(McpAllowlist { servers })
    }

    #[test]
    fn allows_git_status() {
        let gate = PermissionGate::from_registry(&fixture_registry());
        let intent = Intent {
            schema_version: "1".into(),
            intent_id: Uuid::new_v4(),
            kind: IntentKind::ToolRequest,
            summary: "git".into(),
            tool: Some(crate::intent::ToolRequest {
                name: "git.status".into(),
                args: serde_json::json!({}),
            }),
        };
        assert!(matches!(gate.evaluate(&intent), PermissionVerdict::Allow));
    }

    #[test]
    fn denies_unknown_tool() {
        let gate = PermissionGate::from_registry(&fixture_registry());
        let intent = Intent {
            schema_version: "1".into(),
            intent_id: Uuid::new_v4(),
            kind: IntentKind::ToolRequest,
            summary: "test".into(),
            tool: Some(crate::intent::ToolRequest {
                name: "system.rm_rf".into(),
                args: serde_json::json!({}),
            }),
        };
        assert!(matches!(gate.evaluate(&intent), PermissionVerdict::Deny(_)));
    }

    fn test_gate_with_github_mcp() -> PermissionGate {
        let mut servers = HashMap::new();
        servers.insert(
            "github".into(),
            McpServerConfig {
                enabled: true,
                command: "npx".into(),
                args: vec!["@github/github-mcp-server".into()],
                allowed_tools: vec!["search_issues".into(), "list_issues".into(), "get_issue".into()],
                isolation: None,
            },
        );
        PermissionGate::from_registry(&fixture_registry()).with_mcp_allowlist(McpAllowlist { servers })
    }

    #[test]
    fn allows_mcp_github_search_issues() {
        let gate = test_gate_with_github_mcp();
        let intent = CoreIntent::McpProxy {
            mcp_server: "github".into(),
            mcp_tool: "search_issues".into(),
            mcp_args: serde_json::json!({"query": "repo:Ishwanku/RMNG-OS is:open"}),
            metadata: None,
        };
        assert!(matches!(gate.evaluate_core(&intent), PermissionVerdict::Allow));
    }

    #[test]
    fn allows_mcp_git_log() {
        let gate = test_gate_with_mcp();
        let intent = CoreIntent::McpProxy {
            mcp_server: "git".into(),
            mcp_tool: "git.log".into(),
            mcp_args: serde_json::json!({}),
            metadata: None,
        };
        assert!(matches!(gate.evaluate_core(&intent), PermissionVerdict::Allow));
    }

    #[test]
    fn denies_unconfigured_mcp_server() {
        let gate =
            PermissionGate::from_registry(&fixture_registry()).with_mcp_allowlist(McpAllowlist::default());
        let intent = CoreIntent::McpProxy {
            mcp_server: "github".into(),
            mcp_tool: "create_issue".into(),
            mcp_args: serde_json::json!({}),
            metadata: None,
        };
        assert!(matches!(gate.evaluate_core(&intent), PermissionVerdict::Deny(_)));
    }

    fn test_gate_with_fetch_mcp() -> PermissionGate {
        let mut servers = HashMap::new();
        servers.insert(
            "fetch".into(),
            McpServerConfig {
                enabled: true,
                command: "npx".into(),
                args: vec!["-y".into(), "@modelcontextprotocol/server-fetch".into()],
                allowed_tools: vec!["fetch".into()],
                isolation: None,
            },
        );
        PermissionGate::from_registry(&fixture_registry()).with_mcp_allowlist(McpAllowlist { servers })
    }

    #[test]
    fn allows_mcp_fetch() {
        let gate = test_gate_with_fetch_mcp();
        let intent = CoreIntent::McpProxy {
            mcp_server: "fetch".into(),
            mcp_tool: "fetch".into(),
            mcp_args: serde_json::json!({"url": "https://example.com"}),
            metadata: None,
        };
        assert!(matches!(gate.evaluate_core(&intent), PermissionVerdict::Allow));
    }

    fn test_gate_with_markitdown_mcp() -> PermissionGate {
        let mut servers = HashMap::new();
        servers.insert(
            "markitdown".into(),
            McpServerConfig {
                enabled: true,
                command: "uvx".into(),
                args: vec!["markitdown-mcp".into()],
                allowed_tools: vec!["convert_to_markdown".into()],
                isolation: None,
            },
        );
        PermissionGate::from_registry(&fixture_registry()).with_mcp_allowlist(McpAllowlist { servers })
    }

    #[test]
    fn allows_mcp_markitdown() {
        let gate = test_gate_with_markitdown_mcp();
        let intent = CoreIntent::McpProxy {
            mcp_server: "markitdown".into(),
            mcp_tool: "convert_to_markdown".into(),
            mcp_args: serde_json::json!({"uri": "https://example.com/doc.pdf"}),
            metadata: None,
        };
        assert!(matches!(gate.evaluate_core(&intent), PermissionVerdict::Allow));
    }

    #[test]
    fn allows_mcp_github_list_issues() {
        let gate = test_gate_with_github_mcp();
        let intent = CoreIntent::McpProxy {
            mcp_server: "github".into(),
            mcp_tool: "list_issues".into(),
            mcp_args: serde_json::json!({"owner": "Ishwanku", "repo": "RMNG-OS"}),
            metadata: None,
        };
        assert!(matches!(gate.evaluate_core(&intent), PermissionVerdict::Allow));
    }

    #[test]
    fn allows_mcp_github_get_issue() {
        let gate = test_gate_with_github_mcp();
        let intent = CoreIntent::McpProxy {
            mcp_server: "github".into(),
            mcp_tool: "get_issue".into(),
            mcp_args: serde_json::json!({"owner": "Ishwanku", "repo": "RMNG-OS", "issue_number": 1}),
            metadata: None,
        };
        assert!(matches!(gate.evaluate_core(&intent), PermissionVerdict::Allow));
    }

    #[test]
    fn denies_mcp_github_create_issue() {
        let gate = test_gate_with_github_mcp();
        let intent = CoreIntent::McpProxy {
            mcp_server: "github".into(),
            mcp_tool: "create_issue".into(),
            mcp_args: serde_json::json!({}),
            metadata: None,
        };
        assert!(matches!(gate.evaluate_core(&intent), PermissionVerdict::Deny(_)));
    }

    #[test]
    fn allows_mcp_git_diff() {
        let gate = test_gate_with_mcp();
        let intent = CoreIntent::McpProxy {
            mcp_server: "git".into(),
            mcp_tool: "git.diff".into(),
            mcp_args: serde_json::json!({"repo_path": "/tmp/repo"}),
            metadata: None,
        };
        assert!(matches!(gate.evaluate_core(&intent), PermissionVerdict::Allow));
    }

    #[test]
    fn allows_mcp_git_status() {
        let gate = test_gate_with_mcp();
        let intent = CoreIntent::McpProxy {
            mcp_server: "git".into(),
            mcp_tool: "git.status".into(),
            mcp_args: serde_json::json!({"repo_path": "/tmp/repo"}),
            metadata: None,
        };
        assert!(matches!(gate.evaluate_core(&intent), PermissionVerdict::Allow));
    }

    fn test_gate_with_mem0_mcp() -> PermissionGate {
        let mut servers = HashMap::new();
        servers.insert(
            "mem0".into(),
            McpServerConfig {
                enabled: true,
                command: "uvx".into(),
                args: vec!["mem0-mcp-server".into()],
                allowed_tools: vec![
                    "add_memory".into(),
                    "search_memories".into(),
                    "get_memory".into(),
                    "delete_memory".into(),
                ],
                isolation: None,
            },
        );
        PermissionGate::from_registry(&fixture_registry()).with_mcp_allowlist(McpAllowlist { servers })
    }

    #[test]
    fn allows_mcp_mem0_search() {
        let gate = test_gate_with_mem0_mcp();
        let intent = CoreIntent::McpProxy {
            mcp_server: "mem0".into(),
            mcp_tool: "search_memories".into(),
            mcp_args: serde_json::json!({"query": "RMNG", "user_id": "rmng-os"}),
            metadata: None,
        };
        assert!(matches!(gate.evaluate_core(&intent), PermissionVerdict::Allow));
    }

    #[test]
    fn allows_mcp_mem0_add() {
        let gate = test_gate_with_mem0_mcp();
        let intent = CoreIntent::McpProxy {
            mcp_server: "mem0".into(),
            mcp_tool: "add_memory".into(),
            mcp_args: serde_json::json!({"messages": [], "user_id": "rmng-os"}),
            metadata: None,
        };
        assert!(matches!(gate.evaluate_core(&intent), PermissionVerdict::Allow));
    }

    #[test]
    fn denies_mcp_mem0_delete_all() {
        let gate = test_gate_with_mem0_mcp();
        let intent = CoreIntent::McpProxy {
            mcp_server: "mem0".into(),
            mcp_tool: "delete_all_memories".into(),
            mcp_args: serde_json::json!({}),
            metadata: None,
        };
        assert!(matches!(gate.evaluate_core(&intent), PermissionVerdict::Deny(_)));
    }

    #[test]
    fn denies_disallowed_mcp_tool() {
        let gate = test_gate_with_mcp();
        let intent = CoreIntent::McpProxy {
            mcp_server: "git".into(),
            mcp_tool: "git.commit".into(),
            mcp_args: serde_json::json!({}),
            metadata: None,
        };
        assert!(matches!(gate.evaluate_core(&intent), PermissionVerdict::Deny(_)));
    }
}