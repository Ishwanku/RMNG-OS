//! Consolidated production health check (Sprint 28–29).

use rmng_core::{
    agent_registry_check, check_budget_from_audit, daemon_running, socket_path, AuditLog,
    BudgetLevel, CheckLevel, ReadinessReport, RmngConfig,
};
use rmng_nervous::{
    circuit_state_path, health_check_detailed, list_circuit_statuses, reload_from_disk,
    AgentRegistry, NervousConnector,
};
use serde::Serialize;

#[derive(Debug, Clone, Copy, Default)]
pub struct HealthOptions {
    pub json: bool,
    pub quick: bool,
    pub require_daemon: bool,
    pub strict: bool,
}

#[derive(Serialize)]
struct HealthSummary {
    schema_version: u32,
    ok: bool,
    rmngd_running: bool,
    socket_path: String,
    llm_healthy: Option<bool>,
    llm_provider: Option<String>,
    circuits_open: u32,
    audit_valid: Option<bool>,
    budget_level: Option<String>,
    readiness_ok: bool,
    /// Human-readable reasons when unhealthy (Sprint 29).
    failures: Vec<String>,
    readiness: ReadinessReport,
}

fn budget_level_str(level: BudgetLevel) -> &'static str {
    match level {
        BudgetLevel::Ok => "ok",
        BudgetLevel::Warn => "warn",
        BudgetLevel::Deny => "deny",
    }
}

fn evaluate_health(
    readiness_ok: bool,
    audit_valid: Option<bool>,
    llm_healthy: Option<bool>,
    daemon_up: bool,
    circuits_open: u32,
    budget_level: Option<BudgetLevel>,
    opts: HealthOptions,
) -> (bool, Vec<String>) {
    let mut failures = Vec::new();

    if !readiness_ok {
        failures.push("readiness checks failed".into());
    }
    if audit_valid == Some(false) {
        failures.push("audit chain tampered".into());
    }
    if llm_healthy == Some(false) {
        failures.push("llm provider unreachable".into());
    }

    let need_daemon = opts.require_daemon || opts.strict;
    if need_daemon && !daemon_up {
        failures.push("rmngd not running".into());
    }

    if opts.strict {
        if circuits_open > 0 {
            failures.push(format!("{circuits_open} circuit breaker(s) open"));
        }
        if budget_level == Some(BudgetLevel::Deny) {
            failures.push("budget deny threshold exceeded".into());
        }
    }

    let ok = failures.is_empty();
    (ok, failures)
}

pub async fn run_health(opts: HealthOptions) -> i32 {
    reload_from_disk();
    let mut readiness = ReadinessReport::run();
    let agent_result = AgentRegistry::load()
        .map(|r| r.agent_ids().len())
        .map_err(|e| e.to_string());
    readiness.push_check(agent_registry_check(Some(agent_result)));

    let daemon_up = daemon_running();
    let cfg = RmngConfig::load();
    let connector = NervousConnector::from_config(cfg.clone());

    let llm = if opts.quick || !cfg.llm_configured() {
        None
    } else {
        health_check_detailed(connector.config()).await.ok()
    };

    let circuits = list_circuit_statuses();
    let circuits_open = circuits.iter().filter(|c| c.open).count() as u32;

    let audit_valid = AuditLog::new(AuditLog::default_path())
        .verify_chain()
        .ok()
        .map(|v| v.valid);

    let budget = check_budget_from_audit(&cfg);
    let budget_level = budget.as_ref().map(|b| b.level);

    let llm_healthy = llm.as_ref().map(|r| r.healthy);
    let (ok, failures) = evaluate_health(
        readiness.ok,
        audit_valid,
        llm_healthy,
        daemon_up,
        circuits_open,
        budget_level,
        opts,
    );

    if opts.json {
        let out = HealthSummary {
            schema_version: 2,
            ok,
            rmngd_running: daemon_up,
            socket_path: socket_path().display().to_string(),
            llm_healthy,
            llm_provider: llm.as_ref().map(|r| r.provider_id.clone()),
            circuits_open,
            audit_valid,
            budget_level: budget_level.map(budget_level_str).map(str::to_string),
            readiness_ok: readiness.ok,
            failures: failures.clone(),
            readiness,
        };
        println!("{}", serde_json::to_string_pretty(&out).unwrap_or_default());
    } else {
        println!("=== RMNG health ===");
        if opts.strict {
            println!("(strict mode: daemon required, no open circuits, no budget deny)");
        } else if opts.require_daemon {
            println!("(require-daemon: rmngd must be running)");
        }
        println!();
        println!(
            "rmngd:       {} ({})",
            if daemon_up { "running" } else { "stopped" },
            socket_path().display()
        );
        if let Some(ref r) = llm {
            let st = if r.healthy { "healthy" } else { "unreachable" };
            println!("llm:         {} [{st}] {}", r.provider_id, r.model);
        } else if cfg.llm_configured() && opts.quick {
            println!("llm:         skipped (--quick)");
        } else if !cfg.llm_configured() {
            println!("llm:         not configured");
        }
        println!(
            "circuits:    {} open / {} tracked ({})",
            circuits_open,
            circuits.len(),
            circuit_state_path().display()
        );
        match audit_valid {
            Some(true) => println!("audit:       chain valid"),
            Some(false) => println!("audit:       TAMPER DETECTED"),
            None => println!("audit:       not verified"),
        }
        if let Some(ref b) = budget {
            println!(
                "budget:      [{}] {}",
                budget_level_str(b.level),
                b.message
            );
        }
        println!();
        println!("-- readiness --");
        for check in &readiness.checks {
            let tag = match check.level {
                CheckLevel::Ok => "ok",
                CheckLevel::Warn => "warn",
                CheckLevel::Error => "ERROR",
            };
            println!("  [{tag}] {} — {}", check.id, check.message);
        }
        if !failures.is_empty() {
            println!();
            println!("-- failures --");
            for f in &failures {
                println!("  - {f}");
            }
        }
        println!();
        if ok {
            println!("Result: healthy");
        } else {
            println!("Result: unhealthy");
        }
    }

    if ok { 0 } else { 1 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_mode_allows_stopped_daemon() {
        let (ok, failures) = evaluate_health(
            true,
            Some(true),
            Some(true),
            false,
            2,
            Some(BudgetLevel::Deny),
            HealthOptions::default(),
        );
        assert!(ok);
        assert!(failures.is_empty());
    }

    #[test]
    fn require_daemon_fails_when_stopped() {
        let (ok, failures) = evaluate_health(
            true,
            Some(true),
            Some(true),
            false,
            0,
            None,
            HealthOptions {
                require_daemon: true,
                ..Default::default()
            },
        );
        assert!(!ok);
        assert!(failures.iter().any(|f| f.contains("rmngd")));
    }

    #[test]
    fn strict_fails_on_circuits_budget_and_daemon() {
        let opts = HealthOptions {
            strict: true,
            ..Default::default()
        };
        let (ok, _) = evaluate_health(true, Some(true), Some(true), false, 1, None, opts);
        assert!(!ok);

        let (ok, _) = evaluate_health(true, Some(true), Some(true), true, 0, Some(BudgetLevel::Deny), opts);
        assert!(!ok);
    }
}