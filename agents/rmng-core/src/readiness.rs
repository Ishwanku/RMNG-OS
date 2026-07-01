//! Production readiness checks shared by rmngd and `rmng health` (Sprint 28).

use crate::{
    allowlist::McpAllowlist, config::RmngConfig, registry::IntegrationRegistry, AuditLog,
    socket_path,
};
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckLevel {
    Ok,
    Warn,
    Error,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReadinessCheck {
    pub id: String,
    pub level: CheckLevel,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReadinessReport {
    pub ok: bool,
    pub checks: Vec<ReadinessCheck>,
}

impl ReadinessReport {
    pub fn run() -> Self {
        let mut checks = Vec::new();

        let home = rmng_home();
        push_dir(&mut checks, "rmng_home", &home, true);

        let config_path = RmngConfig::config_path();
        let cfg = match std::fs::read_to_string(&config_path) {
            Ok(raw) => match toml::from_str::<RmngConfig>(&raw) {
                Ok(c) => {
                    push_ok(
                        &mut checks,
                        "config",
                        format!("{} parses OK", config_path.display()),
                    );
                    c
                }
                Err(e) => {
                    push_error(
                        &mut checks,
                        "config",
                        format!("{} invalid: {e}", config_path.display()),
                    );
                    RmngConfig::default()
                }
            },
            Err(e) => {
                push_warn(
                    &mut checks,
                    "config",
                    format!("{} missing ({e}); using defaults", config_path.display()),
                );
                RmngConfig::default()
            }
        };

        let sessions = home.join("sessions");
        push_writable(&mut checks, "sessions_dir", &sessions);

        let socket = socket_path();
        if let Some(parent) = socket.parent() {
            push_writable(&mut checks, "socket_dir", parent);
        }

        match IntegrationRegistry::load() {
            Ok(reg) => push_ok(
                &mut checks,
                "integrations",
                format!("{} integration manifest(s)", reg.manifests().len()),
            ),
            Err(e) => push_warn(&mut checks, "integrations", format!("load failed: {e}")),
        }

        if cfg.llm_configured() {
            push_ok(&mut checks, "llm", "provider configured".into());
        } else {
            push_warn(
                &mut checks,
                "llm",
                "llm_provider=none — nervous routing uses mock/plan-only until configured".into(),
            );
        }

        let allowlist_path = McpAllowlist::config_path();
        if allowlist_path.is_file() {
            push_ok(
                &mut checks,
                "mcp_allowlist",
                format!("{}", allowlist_path.display()),
            );
        } else {
            push_warn(
                &mut checks,
                "mcp_allowlist",
                format!("missing {} — run scripts/setup-mcp-allowlist.sh", allowlist_path.display()),
            );
        }

        let audit_path = AuditLog::default_path();
        if audit_path.is_file() {
            let log = AuditLog::new(audit_path.clone());
            match log.verify_chain() {
                Ok(v) if v.valid => push_ok(
                    &mut checks,
                    "audit",
                    format!("{} valid ({} entries)", audit_path.display(), v.entries),
                ),
                Ok(_) => push_error(&mut checks, "audit", "audit chain tampered".into()),
                Err(e) => push_warn(&mut checks, "audit", format!("verify failed: {e}")),
            }
        } else {
            push_ok(
                &mut checks,
                "audit",
                format!("{} will be created on first dispatch", audit_path.display()),
            );
        }

        push_ok(
            &mut checks,
            "auto_continue",
            format!(
                "max_steps={} timeout_secs={}",
                cfg.auto_continue.max_steps, cfg.auto_continue.timeout_secs
            ),
        );

        let ok = !checks.iter().any(|c| c.level == CheckLevel::Error);
        Self { ok, checks }
    }

    pub fn push_check(&mut self, check: ReadinessCheck) {
        if check.level == CheckLevel::Error {
            self.ok = false;
        }
        self.checks.push(check);
    }
}

/// Merge agent-registry results from rmng-nervous (kept out of rmng-core to avoid cycles).
pub fn agent_registry_check(agent_count: Option<Result<usize, String>>) -> ReadinessCheck {
    match agent_count {
        Some(Ok(n)) if n == 0 => ReadinessCheck {
            id: "agents".into(),
            level: CheckLevel::Error,
            message: "agent registry empty — set RMNG_PROJECT_ROOT".into(),
        },
        Some(Ok(n)) => ReadinessCheck {
            id: "agents".into(),
            level: CheckLevel::Ok,
            message: format!("{n} agent definition(s) loaded"),
        },
        Some(Err(e)) => ReadinessCheck {
            id: "agents".into(),
            level: CheckLevel::Error,
            message: format!("registry load failed: {e}"),
        },
        None => ReadinessCheck {
            id: "agents".into(),
            level: CheckLevel::Warn,
            message: "agent registry not checked".into(),
        },
    }
}

fn rmng_home() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".rmng");
    }
    PathBuf::from(".rmng")
}

fn push_ok(checks: &mut Vec<ReadinessCheck>, id: &str, message: String) {
    checks.push(ReadinessCheck {
        id: id.into(),
        level: CheckLevel::Ok,
        message,
    });
}

fn push_warn(checks: &mut Vec<ReadinessCheck>, id: &str, message: String) {
    checks.push(ReadinessCheck {
        id: id.into(),
        level: CheckLevel::Warn,
        message,
    });
}

fn push_error(checks: &mut Vec<ReadinessCheck>, id: &str, message: String) {
    checks.push(ReadinessCheck {
        id: id.into(),
        level: CheckLevel::Error,
        message,
    });
}

fn push_dir(checks: &mut Vec<ReadinessCheck>, id: &str, path: &Path, required: bool) {
    if path.is_dir() {
        push_ok(checks, id, format!("{}", path.display()));
    } else if required {
        push_error(checks, id, format!("missing directory {}", path.display()));
    } else {
        push_warn(checks, id, format!("missing {}", path.display()));
    }
}

fn push_writable(checks: &mut Vec<ReadinessCheck>, id: &str, path: &Path) {
    if !path.exists() {
        if std::fs::create_dir_all(path).is_ok() {
            push_ok(checks, id, format!("created {}", path.display()));
            return;
        }
        push_error(checks, id, format!("cannot create {}", path.display()));
        return;
    }
    let probe = path.join(".rmng-write-probe");
    match std::fs::write(&probe, b"ok") {
        Ok(()) => {
            let _ = std::fs::remove_file(&probe);
            push_ok(checks, id, format!("{} writable", path.display()));
        }
        Err(e) => push_error(checks, id, format!("{} not writable: {e}", path.display())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readiness_report_has_checks() {
        let report = ReadinessReport::run();
        assert!(!report.checks.is_empty());
        assert!(report.checks.iter().any(|c| c.id == "config"));
    }
}