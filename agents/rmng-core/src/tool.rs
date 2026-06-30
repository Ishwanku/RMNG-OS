use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolResult {
    pub success: bool,
    pub output: String,
    pub exit_code: Option<i32>,
}
