use crate::intent::{Intent, IntentKind, ToolRequest};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionVerdict {
    Allow,
    Deny(String),
}

#[derive(Clone)]
pub struct PermissionGate {
    allowed_tools: Vec<String>,
}

impl Default for PermissionGate {
    fn default() -> Self {
        Self {
            allowed_tools: vec![
                "kernel.status".into(),
                "kernel.build".into(),
                "kernel.apply_patches".into(),
            ],
        }
    }
}

impl PermissionGate {
    pub fn new(allowed_tools: Vec<String>) -> Self {
        Self { allowed_tools }
    }

    pub fn evaluate(&self, intent: &Intent) -> PermissionVerdict {
        match intent.kind {
            IntentKind::Plan | IntentKind::Clarify | IntentKind::Complete => PermissionVerdict::Allow,
            IntentKind::ToolRequest => {
                let Some(tool) = &intent.tool else {
                    return PermissionVerdict::Deny("tool_request missing tool field".into());
                };
                self.evaluate_tool(tool)
            }
        }
    }

    fn evaluate_tool(&self, tool: &ToolRequest) -> PermissionVerdict {
        if self.allowed_tools.iter().any(|t| t == &tool.name) {
            PermissionVerdict::Allow
        } else {
            PermissionVerdict::Deny(format!("tool not allowed: {}", tool.name))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intent::{Intent, IntentKind};
    use uuid::Uuid;

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
}
