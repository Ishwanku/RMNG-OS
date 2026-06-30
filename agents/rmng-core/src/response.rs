use crate::intent::IntentKind;
use crate::tool::ToolResult;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandleResponse {
    pub ok: bool,
    pub kind: Option<IntentKind>,
    pub tool_result: Option<ToolResult>,
    pub error: Option<String>,
}

impl HandleResponse {
    pub fn success(kind: IntentKind, tool_result: Option<ToolResult>) -> Self {
        Self {
            ok: true,
            kind: Some(kind),
            tool_result,
            error: None,
        }
    }

    pub fn failure(error: impl Into<String>) -> Self {
        Self {
            ok: false,
            kind: None,
            tool_result: None,
            error: Some(error.into()),
        }
    }
}
