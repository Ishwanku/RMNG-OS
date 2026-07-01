//! RMNG-OS runtime core — intent parsing, permissions, tool dispatch, audit, IPC, config.

pub mod allowlist;
pub mod audit;
pub mod budget;
pub mod config;
pub mod cost_rollup;
pub mod dispatch;
pub mod error;
pub mod intent;
pub mod ipc;
pub mod permission;
pub mod registry;
pub mod response;
pub mod session;
pub mod tool;
pub mod tools;
pub mod validator;

pub use allowlist::{McpAllowlist, McpServerConfig};
pub use audit::{
    AuditCategory, AuditEntry, AuditLog, AuditTrack, ChainVerifyResult, AUDIT_GENESIS_HASH,
    AUDIT_SCHEMA_VERSION,
};
pub use rmng_mcp::{IsolationLimits, IsolationReport, McpCallResult};
pub use registry::{IntegrationManifest, IntegrationRegistry, ToolManifest};
pub use session::{
    build_tool_result_record, persist_dispatch_to_session, session_ttl_days, AgentSession,
    HandoffRecord, LlmCallRecord, SessionError, SessionLoadOptions, SessionStore,
    ToolResultRecord, DEFAULT_SESSION_TTL_DAYS, MAX_TOOL_OUTPUT_LEN,
};
pub use validator::IntentValidator;
pub use budget::{
    check_budget, check_budget_for_agent, check_budget_from_audit, check_budget_from_audit_for_agent,
    spent_today_for_agent, spent_today_usd, BudgetCheckResult, BudgetLevel,
};
pub use config::{
    parse_provider_str, AgentLlmOverride, BudgetEnforceMode, LlmBudgetConfig, LlmConfig,
    LlmConfigEntry, LlmProfile, LlmProvider, LlmProviderKind, RmngConfig,
};
pub use cost_rollup::{
    rollup_llm_costs, rollup_recent_days, CostRollupReport, EntityCost, PeriodCost, RankedEntityCost,
};
pub use dispatch::Runtime;
pub use error::RmngError;
pub use intent::{
    CoreIntent, Intent, IntentKind, Metadata, ToolRequest, CORE_INTENT_SCHEMA_VERSION,
};

/// Parse IPC payload as v2 core intent or fall back to v1 intent envelope.
pub fn parse_incoming(json: &str) -> Result<IncomingIntent, RmngError> {
    let value: serde_json::Value = serde_json::from_str(json)?;
    if value.get("action").is_some() {
        Ok(IncomingIntent::Core(CoreIntent::parse(json)?))
    } else {
        Ok(IncomingIntent::V1(Intent::parse(json)?))
    }
}

/// IPC envelope: v1 legacy intent or v2 core intent.
#[derive(Debug, Clone)]
pub enum IncomingIntent {
    V1(Intent),
    Core(CoreIntent),
}
pub use ipc::{daemon_running, send_intent_json, socket_path};
pub use permission::{PermissionGate, PermissionVerdict};
pub use response::HandleResponse;
pub use tool::ToolResult;
