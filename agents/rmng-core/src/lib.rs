//! RMNG-OS runtime core — intent parsing, permissions, tool dispatch.

pub mod error;
pub mod intent;
pub mod permission;

pub use error::RmngError;
pub use intent::{Intent, IntentKind, ToolRequest};
pub use permission::{PermissionGate, PermissionVerdict};
