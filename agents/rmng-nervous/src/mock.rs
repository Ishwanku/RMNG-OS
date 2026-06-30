use rmng_core::{Intent, IntentKind, RmngError};
use uuid::Uuid;

/// Nervous-system stub when no LLM provider is configured (BYO-LLM default).
pub fn mock_intent(prompt: &str) -> Intent {
    Intent {
        schema_version: "1".into(),
        intent_id: Uuid::new_v4(),
        kind: IntentKind::Plan,
        summary: format!(
            "[mock nervous-system] no LLM provider configured — received: {prompt}"
        ),
        tool: None,
    }
}

pub fn mock_intent_for_tool(prompt: &str, tool_name: &str) -> Result<Intent, RmngError> {
    Ok(Intent {
        schema_version: "1".into(),
        intent_id: Uuid::new_v4(),
        kind: IntentKind::ToolRequest,
        summary: format!("[mock nervous-system] {prompt}"),
        tool: Some(rmng_core::ToolRequest {
            name: tool_name.into(),
            args: serde_json::json!({}),
        }),
    })
}
