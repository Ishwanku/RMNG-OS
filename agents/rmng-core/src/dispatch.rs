use crate::audit::{AuditEntry, AuditLog};
use crate::intent::{Intent, IntentKind};
use crate::permission::{PermissionGate, PermissionVerdict};
use crate::tool::ToolResult;
use crate::tools;
use crate::RmngError;
use chrono::Utc;
use tracing::{info, warn};

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
        match self.gate.evaluate(intent) {
            PermissionVerdict::Deny(reason) => {
                self.log(intent, "deny", &reason);
                return Err(RmngError::PermissionDenied(reason));
            }
            PermissionVerdict::Allow => {}
        }

        match intent.kind {
            IntentKind::Plan | IntentKind::Clarify | IntentKind::Complete => {
                self.log(intent, &format!("{:?}", intent.kind), "ok");
                Ok(None)
            }
            IntentKind::ToolRequest => {
                let tool = intent
                    .tool
                    .as_ref()
                    .ok_or_else(|| RmngError::InvalidIntent("missing tool".into()))?;
                info!(tool = %tool.name, "dispatching tool");
                let result = tools::dispatch(&tool.name, &tool.args).await?;
                let outcome = if result.success { "ok" } else { "fail" };
                self.log(intent, &tool.name, outcome);
                Ok(Some(result))
            }
        }
    }

    fn log(&self, intent: &Intent, action: &str, outcome: &str) {
        let entry = AuditEntry {
            timestamp: Utc::now(),
            intent_id: intent.intent_id,
            action: action.to_string(),
            outcome: outcome.to_string(),
            detail: Some(intent.summary.clone()),
        };
        if let Err(e) = self.audit.append(&entry) {
            warn!(error = %e, "audit log write failed");
        }
    }
}
