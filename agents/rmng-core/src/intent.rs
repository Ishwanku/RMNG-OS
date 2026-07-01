use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Core intent schema version for poly-intent envelopes (`core-intent.schema.json`).
pub const CORE_INTENT_SCHEMA_VERSION: &str = "2";

// ---------------------------------------------------------------------------
// v2 — Poly-intent core envelope (ADR-015)
// ---------------------------------------------------------------------------

/// Optional correlation and nervous-system context carried on any intent variant.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Metadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skill_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub handoff_from: Option<String>,
}

/// Internally tagged poly-intent: native tools, MCP proxy, or plan-only reasoning.
///
/// Serialized with top-level `"action"` discriminator per `core-intent.schema.json`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "action", deny_unknown_fields)]
pub enum CoreIntent {
    /// Execute a native RMNG tool through `rmngd` + `PermissionGate`.
    #[serde(rename = "tool.execute")]
    ToolExecute {
        target: String,
        parameters: serde_json::Value,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        metadata: Option<Metadata>,
    },
    /// Proxy an allowlisted MCP server tool (Phase 6b bridge).
    #[serde(rename = "mcp.proxy")]
    McpProxy {
        mcp_server: String,
        mcp_tool: String,
        mcp_args: serde_json::Value,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        metadata: Option<Metadata>,
    },
    /// Reasoning-only transition; no execution fields.
    #[serde(rename = "plan.only")]
    PlanOnly {
        reasoning: String,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        metadata: Option<Metadata>,
    },
}

impl CoreIntent {
    pub fn parse(json: &str) -> Result<Self, crate::RmngError> {
        serde_json::from_str(json).map_err(crate::RmngError::from)
    }

    pub fn is_executable(&self) -> bool {
        matches!(self, CoreIntent::ToolExecute { .. } | CoreIntent::McpProxy { .. })
    }

    pub fn metadata(&self) -> Option<&Metadata> {
        match self {
            CoreIntent::ToolExecute { metadata, .. }
            | CoreIntent::McpProxy { metadata, .. }
            | CoreIntent::PlanOnly { metadata, .. } => metadata.as_ref(),
        }
    }
}

// ---------------------------------------------------------------------------
// v1 — Legacy intent envelope (backward compatible)
// ---------------------------------------------------------------------------

/// JSON intent from the nervous-system layer (LLM). Never contains shell commands directly.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Intent {
    pub schema_version: String,
    pub intent_id: Uuid,
    pub kind: IntentKind,
    pub summary: String,
    pub tool: Option<ToolRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IntentKind {
    Plan,
    ToolRequest,
    Clarify,
    Complete,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolRequest {
    pub name: String,
    pub args: serde_json::Value,
}

impl Intent {
    pub fn parse(json: &str) -> Result<Self, crate::RmngError> {
        let intent: Intent = serde_json::from_str(json)?;
        if intent.schema_version != "1" {
            return Err(crate::RmngError::InvalidIntent(format!(
                "unsupported schema_version: {}",
                intent.schema_version
            )));
        }
        Ok(intent)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_kernel_status_intent() {
        let json = include_str!("../../schemas/kernel-status.intent.json");
        let intent = Intent::parse(json).expect("valid intent");
        assert_eq!(intent.kind, IntentKind::ToolRequest);
        assert_eq!(intent.tool.as_ref().unwrap().name, "kernel.status");
    }

    #[test]
    fn parses_tool_execute_core_intent() {
        let json = r#"{
            "action": "tool.execute",
            "target": "kernel.status",
            "parameters": {},
            "metadata": { "skill_name": "kernel-build" }
        }"#;
        let intent = CoreIntent::parse(json).expect("valid core intent");
        assert!(matches!(intent, CoreIntent::ToolExecute { .. }));
        assert!(intent.is_executable());
        assert_eq!(
            intent.metadata().and_then(|m| m.skill_name.as_deref()),
            Some("kernel-build")
        );
    }

    #[test]
    fn parses_mcp_proxy_core_intent() {
        let json = r#"{
            "action": "mcp.proxy",
            "mcp_server": "github",
            "mcp_tool": "create_issue",
            "mcp_args": { "title": "test" }
        }"#;
        let intent = CoreIntent::parse(json).expect("valid mcp proxy");
        assert!(matches!(intent, CoreIntent::McpProxy { .. }));
        assert!(intent.is_executable());
    }

    #[test]
    fn parses_plan_only_core_intent() {
        let json = r#"{
            "action": "plan.only",
            "reasoning": "Need slim config before rebuild."
        }"#;
        let intent = CoreIntent::parse(json).expect("valid plan");
        assert!(matches!(intent, CoreIntent::PlanOnly { .. }));
        assert!(!intent.is_executable());
    }

    #[test]
    fn rejects_unknown_fields_on_core_intent() {
        let json = r#"{
            "action": "tool.execute",
            "target": "git.status",
            "parameters": {},
            "shell": "rm -rf /"
        }"#;
        assert!(CoreIntent::parse(json).is_err());
    }

    #[test]
    fn rejects_invalid_action() {
        let json = r#"{
            "action": "shell.exec",
            "command": "whoami"
        }"#;
        assert!(CoreIntent::parse(json).is_err());
    }
}