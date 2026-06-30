//! RMNG-OS runtime core — intent parsing, permissions, tool dispatch, audit, IPC, config.

pub mod audit;
pub mod config;
pub mod dispatch;
pub mod error;
pub mod intent;
pub mod ipc;
pub mod permission;
pub mod response;
pub mod tool;
pub mod tools;

pub use audit::{AuditEntry, AuditLog};
pub use config::{LlmConfig, LlmProvider, RmngConfig};
pub use dispatch::Runtime;
pub use error::RmngError;
pub use intent::{Intent, IntentKind, ToolRequest};
pub use ipc::{daemon_running, send_intent_json, socket_path};
pub use permission::{PermissionGate, PermissionVerdict};
pub use response::HandleResponse;
pub use tool::ToolResult;
