use crate::allowlist::{McpAllowlist, McpServerConfig};
use crate::intent::{CoreIntent, Intent, IntentKind};

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
        Self {
            allowed_tools: vec![
                "kernel.status".into(),
                "kernel.build".into(),
                "kernel.apply_patches".into(),
                "git.status".into(),
            ],
            mcp_allowlist: McpAllowlist::load().unwrap_or_default(),
        }
    }
}

impl PermissionGate {
    pub fn new(allowed_tools: Vec<String>) -> Self {
        Self {
            allowed_tools,
            mcp_allowlist: McpAllowlist::load().unwrap_or_default(),
        }
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
    use std::collections::HashMap;
    use uuid::Uuid;

    fn test_gate_with_mcp() -> PermissionGate {
        let mut servers = HashMap::new();
        servers.insert(
            "git".into(),
            McpServerConfig {
                enabled: true,
                command: "uvx".into(),
                args: vec!["mcp-server-git".into()],
                allowed_tools: vec!["git.log".into()],
            },
        );
        PermissionGate::default().with_mcp_allowlist(McpAllowlist { servers })
    }

    #[test]
    fn allows_git_status() {
        let gate = PermissionGate::default();
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
        let gate = PermissionGate::default();
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
        let gate = PermissionGate::default().with_mcp_allowlist(McpAllowlist::default());
        let intent = CoreIntent::McpProxy {
            mcp_server: "github".into(),
            mcp_tool: "create_issue".into(),
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