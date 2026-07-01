use crate::audit::{AuditEntry, AuditLog, AuditTrack};
use crate::intent::{CoreIntent, Intent, IntentKind};
use crate::permission::{PermissionGate, PermissionVerdict};
use crate::registry::IntegrationRegistry;
use crate::response::HandleResponse;
use crate::tool::ToolResult;
use crate::tools;
use crate::validator::IntentValidator;
use crate::RmngError;
use chrono::Utc;
use rmng_mcp::call_tool as mcp_call_tool;
use std::time::Instant;
use tracing::{info, warn};
use uuid::Uuid;

#[derive(Clone)]
pub struct Runtime {
    gate: PermissionGate,
    audit: AuditLog,
    validator: IntentValidator,
}

impl Default for Runtime {
    fn default() -> Self {
        Self::bootstrap().unwrap_or_else(|e| {
            tracing::warn!(error = %e, "runtime bootstrap failed — degraded mode");
            let registry =
                IntegrationRegistry::load_from(std::path::Path::new("/nonexistent")).unwrap();
            let gate = PermissionGate::from_registry(&registry);
            let validator = IntentValidator::new(registry).expect("validator in degraded mode");
            Self {
                gate,
                audit: AuditLog::new(AuditLog::default_path()),
                validator,
            }
        })
    }
}

impl Runtime {
    /// Load integration manifests, validator, and permission gate at startup.
    pub fn bootstrap() -> Result<Self, RmngError> {
        let registry = IntegrationRegistry::load()?;
        for name in registry.allowed_tool_names() {
            if !tools::registered_tools().contains(&name.as_str()) {
                tracing::warn!(
                    tool = %name,
                    "integration manifest has no registered handler in tools/"
                );
            }
        }
        let gate = PermissionGate::from_registry(&registry);
        let validator = IntentValidator::new(registry)?;
        Ok(Self {
            gate,
            audit: AuditLog::new(AuditLog::default_path()),
            validator,
        })
    }

    pub fn new(gate: PermissionGate, audit: AuditLog, validator: IntentValidator) -> Self {
        Self {
            gate,
            audit,
            validator,
        }
    }

    pub fn validator(&self) -> &IntentValidator {
        &self.validator
    }

    pub fn gate(&self) -> &PermissionGate {
        &self.gate
    }

    pub async fn handle(&self, intent: &Intent) -> Result<Option<ToolResult>, RmngError> {
        Ok(self.handle_response(intent).await?.tool_result)
    }

    pub async fn handle_response(&self, intent: &Intent) -> Result<HandleResponse, RmngError> {
        let started = Instant::now();
        match self.gate.evaluate(intent) {
            PermissionVerdict::Deny(reason) => {
                self.log_v1(intent, "deny", "deny", started, None, Some(reason.clone()));
                return Ok(HandleResponse::failure(reason));
            }
            PermissionVerdict::Allow => {}
        }

        match intent.kind {
            IntentKind::Plan | IntentKind::Clarify | IntentKind::Complete => {
                self.log_v1(
                    intent,
                    &format!("{:?}", intent.kind),
                    "ok",
                    started,
                    Some(AuditTrack::Plan),
                    None,
                );
                Ok(HandleResponse::success(intent.kind.clone(), None))
            }
            IntentKind::ToolRequest => {
                let tool = intent
                    .tool
                    .as_ref()
                    .ok_or_else(|| RmngError::InvalidIntent("missing tool".into()))?;
                if let Err(e) = self
                    .validator
                    .validate_tool_parameters(&tool.name, &tool.args)
                {
                    self.log_v1(
                        intent,
                        &tool.name,
                        "invalid",
                        started,
                        Some(AuditTrack::Native),
                        Some(e.to_string()),
                    );
                    return Ok(HandleResponse::failure(e.to_string()));
                }
                info!(tool = %tool.name, "dispatching native tool");
                let result = tools::dispatch(&tool.name, &tool.args).await?;
                let outcome = if result.success { "ok" } else { "fail" };
                self.log_v1(
                    intent,
                    &tool.name,
                    outcome,
                    started,
                    Some(AuditTrack::Native),
                    None,
                );
                Ok(HandleResponse::success(intent.kind.clone(), Some(result)))
            }
        }
    }

    pub async fn handle_incoming(
        &self,
        incoming: &crate::IncomingIntent,
    ) -> Result<HandleResponse, RmngError> {
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
        let started = Instant::now();
        if let Err(e) = self.validator.validate(intent) {
            self.log_core(intent, "validate", "invalid", started, None, Some(e.to_string()));
            return Ok(HandleResponse::failure(e.to_string()));
        }

        match self.gate.evaluate_core(intent) {
            PermissionVerdict::Deny(reason) => {
                self.log_core(intent, "deny", "deny", started, None, Some(reason.clone()));
                return Ok(HandleResponse::failure(reason));
            }
            PermissionVerdict::Allow => {}
        }

        match intent {
            CoreIntent::PlanOnly { reasoning, .. } => {
                self.log_core(intent, "plan.only", "ok", started, Some(AuditTrack::Plan), None);
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
                self.log_core(intent, target, outcome, started, Some(AuditTrack::Native), None);
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
                let output = mcp_call_tool(&cfg.command, &cfg.args, mcp_tool, mcp_args)
                    .await
                    .map_err(|e| RmngError::ToolFailed(e.to_string()))?;
                let action = format!("mcp.proxy:{mcp_server}.{mcp_tool}");
                self.log_core(intent, &action, "ok", started, Some(AuditTrack::Mcp), None);
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

    fn log_v1(
        &self,
        intent: &Intent,
        action: &str,
        outcome: &str,
        started: Instant,
        track: Option<AuditTrack>,
        detail_override: Option<String>,
    ) {
        let entry = AuditEntry {
            timestamp: Utc::now(),
            intent_id: intent.intent_id,
            trace_id: None,
            skill_name: None,
            track,
            duration_ms: Some(started.elapsed().as_millis() as u64),
            mcp_server: None,
            action: action.to_string(),
            outcome: outcome.to_string(),
            detail: detail_override.or_else(|| Some(intent.summary.clone())),
        };
        self.append_audit(&entry);
    }

    fn log_core(
        &self,
        intent: &CoreIntent,
        action: &str,
        outcome: &str,
        started: Instant,
        track: Option<AuditTrack>,
        detail_override: Option<String>,
    ) {
        let meta = intent.metadata();
        let trace_id = meta.and_then(|m| m.trace_id.clone());
        let skill_name = meta.and_then(|m| m.skill_name.clone());
        let intent_id = trace_id
            .as_deref()
            .and_then(|t| Uuid::parse_str(t).ok())
            .unwrap_or_else(Uuid::new_v4);
        let mcp_server = match intent {
            CoreIntent::McpProxy { mcp_server, .. } => Some(mcp_server.clone()),
            _ => None,
        };
        let detail = detail_override.or_else(|| match intent {
            CoreIntent::PlanOnly { reasoning, .. } => Some(reasoning.clone()),
            CoreIntent::ToolExecute { target, .. } => Some(format!("target={target}")),
            CoreIntent::McpProxy {
                mcp_server,
                mcp_tool,
                ..
            } => Some(format!("{mcp_server}.{mcp_tool}")),
        });
        let entry = AuditEntry {
            timestamp: Utc::now(),
            intent_id,
            trace_id,
            skill_name,
            track,
            duration_ms: Some(started.elapsed().as_millis() as u64),
            mcp_server,
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