use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
}
