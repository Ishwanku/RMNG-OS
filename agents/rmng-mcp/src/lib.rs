mod client;
mod error;
mod isolation;

pub use client::{call_tool, call_tool_isolated, wire_tool_name, McpCallResult};
pub use error::McpError;
pub use isolation::{IsolationLimits, IsolationReport};