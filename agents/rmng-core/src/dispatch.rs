use crate::audit::{AuditEntry, AuditLog};
use crate::intent::{CoreIntent, Intent, IntentKind};
use crate::permission::{PermissionGate, PermissionVerdict};
use crate::response::HandleResponse;
use crate::tool::ToolResult;
use crate::tools;
use crate::RmngError;
use chrono::Utc;
use rmng_mcp::call_tool as mcp_call_tool;
use tracing::{info, warn};
use uuid::Uuid;

#[derive(Clone)]
pub struct Runtime {
    gate: PermissionGate,
    audit: AuditLog,
}

impl Default for Runtime {
    fn default() -> Self {
        Self {
            gate: PermissionGate::default(),
            audit: AuditLog::new(AuditLog::default_path()),
        }
    }
}

impl Runtime {
    pub fn new(gate: PermissionGate, audit: AuditLog) -> Self {
        Self { gate, audit }
    }

    pub async fn handle(&self, intent: &Intent) -> Result<Option<ToolResult>, RmngError> {
        Ok(self.handle_response(intent).await?.tool_result)
    }

    pub async fn handle_response(&self, intent: &Intent) -> Result<HandleResponse, RmngError> {
        match self.gate.evaluate(intent) {
            PermissionVerdict::Deny(reason) => {
                self.log_v1(intent, "deny", &reason);
                return Ok(HandleResponse::failure(reason));
            }
            PermissionVerdict::Allow => {}
        }

        match intent.kind {
            IntentKind::Plan | IntentKind::Clarify | IntentKind::Complete => {
                self.log_v1(intent, &format!("{:?}", intent.kind), "ok");
                Ok(HandleResponse::success(intent.kind.clone(), None))
            }
            IntentKind::ToolRequest => {
                let tool = intent
                    .tool
                    .as_ref()
                    .ok_or_else(|| RmngError::InvalidIntent("missing tool".into()))?;
                info!(tool = %tool.name, "dispatching native tool");
                let result = tools::dispatch(&tool.name, &tool.args).await?;
                let outcome = if result.success { "ok" } else { "fail" };
                self.log_v1(intent, &tool.name, outcome);
                Ok(HandleResponse::success(intent.kind.clone(), Some(result)))
            }
        }
    }

    pub async fn handle_incoming(&self, incoming: &crate::IncomingIntent) -> Result<HandleResponse, RmngError> {
        match incoming {
            crate::IncomingIntent::V1(intent) => self.handle_response(intent).await,
            crate::IncomingIntent::Core(intent) => self.handle_core_response(intent).await,
        }
    }

    pub async fn handle_core(&self, intent: &CoreIntent) -> Result<Option<ToolResult>, RmngError> {
        Ok(self.handle_core_response(intent).await?.tool_result)
    }

    pub async fn handle_core_response(
        &self,
        intent: &CoreIntent,
    ) -> Result<HandleResponse, RmngError> {
        match self.gate.evaluate_core(intent) {
            PermissionVerdict::Deny(reason) => {
                self.log_core(intent, "deny", &reason);
                return Ok(HandleResponse::failure(reason));
            }
            PermissionVerdict::Allow => {}
        }

        match intent {
            CoreIntent::PlanOnly { reasoning, .. } => {
                self.log_core(intent, "plan.only", "ok");
                Ok(HandleResponse::core_success(
                    "plan.only",
                    Some(ToolResult {
                        success: true,
                        output: reasoning.clone(),
                        exit_code: Some(0),
                    }),
                ))
            }
            CoreIntent::ToolExecute {
                target,
                parameters,
                ..
            } => {
                info!(tool = %target, "dispatching native tool (v2)");
                let result = tools::dispatch(target, parameters).await?;
                let outcome = if result.success { "ok" } else { "fail" };
                self.log_core(intent, target, outcome);
                Ok(HandleResponse::core_success("tool.execute", Some(result)))
            }
            CoreIntent::McpProxy {
                mcp_server,
                mcp_tool,
                mcp_args,
                ..
            } => {
                let cfg = self
                    .gate
                    .mcp_server_config(mcp_server)
                    .ok_or_else(|| {
                        RmngError::PermissionDenied(format!("mcp server not available: {mcp_server}"))
                    })?;
                info!(
                    server = %mcp_server,
                    tool = %mcp_tool,
                    "dispatching mcp proxy"
                );
                let output = mcp_call_tool(
                    &cfg.command,
                    &cfg.args,
                    mcp_tool,
                    mcp_args,
                )
                .await
                .map_err(|e| RmngError::ToolFailed(e.to_string()))?;
                let action = format!("mcp.proxy:{mcp_server}.{mcp_tool}");
                self.log_core(intent, &action, "ok");
                Ok(HandleResponse::core_success(
                    "mcp.proxy",
                    Some(ToolResult {
                        success: true,
                        output,
                        exit_code: Some(0),
                    }),
                ))
            }
        }
    }

    fn log_v1(&self, intent: &Intent, action: &str, outcome: &str) {
        let entry = AuditEntry {
            timestamp: Utc::now(),
            intent_id: intent.intent_id,
            action: action.to_string(),
            outcome: outcome.to_string(),
            detail: Some(intent.summary.clone()),
        };
        self.append_audit(&entry);
    }

    fn log_core(&self, intent: &CoreIntent, action: &str, outcome: &str) {
        let trace = intent
            .metadata()
            .and_then(|m| m.trace_id.clone())
            .and_then(|t| Uuid::parse_str(&t).ok())
            .unwrap_or_else(Uuid::new_v4);
        let detail = match intent {
            CoreIntent::PlanOnly { reasoning, .. } => Some(reasoning.clone()),
            CoreIntent::ToolExecute { target, .. } => Some(format!("target={target}")),
            CoreIntent::McpProxy {
                mcp_server,
                mcp_tool,
                ..
            } => Some(format!("{mcp_server}.{mcp_tool}")),
        };
        let entry = AuditEntry {
            timestamp: Utc::now(),
            intent_id: trace,
            action: action.to_string(),
            outcome: outcome.to_string(),
            detail,
        };
        self.append_audit(&entry);
    }

    fn append_audit(&self, entry: &AuditEntry) {
        if let Err(e) = self.audit.append(entry) {
            warn!(error = %e, "audit log write failed");
        }
    }
}