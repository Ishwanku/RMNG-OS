use crate::intent::IntentKind;
use crate::tool::ToolResult;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandleResponse {
    pub ok: bool,
    pub kind: Option<IntentKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    pub tool_result: Option<ToolResult>,
    pub error: Option<String>,
}

impl HandleResponse {
    pub fn success(kind: IntentKind, tool_result: Option<ToolResult>) -> Self {
        Self {
            ok: true,
            kind: Some(kind),
            action: None,
            tool_result,
            error: None,
        }
    }

    pub fn core_success(action: impl Into<String>, tool_result: Option<ToolResult>) -> Self {
        Self {
            ok: true,
            kind: None,
            action: Some(action.into()),
            tool_result,
            error: None,
        }
    }

    pub fn failure(error: impl Into<String>) -> Self {
        Self {
            ok: false,
            kind: None,
            action: None,
            tool_result: None,
            error: Some(error.into()),
        }
    }
}