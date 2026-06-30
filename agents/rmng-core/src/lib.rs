//! RMNG-OS runtime core — intent parsing, permissions, tool dispatch, audit.

pub mod audit;
pub mod dispatch;
pub mod error;
pub mod intent;
pub mod permission;
pub mod tool;
pub mod tools;

pub use audit::{AuditEntry, AuditLog};
pub use dispatch::Runtime;
pub use error::RmngError;
pub use intent::{Intent, IntentKind, ToolRequest};
pub use permission::{PermissionGate, PermissionVerdict};
pub use tool::ToolResult;
