//! Consolidated production health check (Sprint 28).

use rmng_core::{
    agent_registry_check, check_budget_from_audit, daemon_running, socket_path, AuditLog,
    CheckLevel, ReadinessReport, RmngConfig,
};
use rmng_nervous::{
    circuit_state_path, health_check_detailed, list_circuit_statuses, reload_from_disk,
    AgentRegistry, NervousConnector,
};
use serde::Serialize;

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
    readiness: ReadinessReport,
}

pub async fn run_health(json: bool, quick: bool) -> i32 {
    reload_from_disk();
    let mut readiness = ReadinessReport::run();
    let agent_result = AgentRegistry::load()
        .map(|r| r.agent_ids().len())
        .map_err(|e| e.to_string());
    readiness.push_check(agent_registry_check(Some(agent_result)));

    let daemon_up = daemon_running();
    let cfg = RmngConfig::load();
    let connector = NervousConnector::from_config(cfg.clone());

    let llm = if quick || !cfg.llm_configured() {
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
    let budget_level = budget.as_ref().map(|b| format!("{:?}", b.level));

    let llm_healthy = llm.as_ref().map(|r| r.healthy);
    let mut ok = readiness.ok && audit_valid.unwrap_or(true);
    if let Some(false) = llm_healthy {
        ok = false;
    }
    if daemon_up == false {
        // CLI-only workflows are valid; warn in human mode, don't fail health by default.
    }

    if json {
        let out = HealthSummary {
            schema_version: 1,
            ok,
            rmngd_running: daemon_up,
            socket_path: socket_path().display().to_string(),
            llm_healthy,
            llm_provider: llm.as_ref().map(|r| r.provider_id.clone()),
            circuits_open,
            audit_valid,
            budget_level,
            readiness_ok: readiness.ok,
            readiness,
        };
        println!("{}", serde_json::to_string_pretty(&out).unwrap_or_default());
    } else {
        println!("=== RMNG health ===");
        println!();
        println!(
            "rmngd:       {} ({})",
            if daemon_up { "running" } else { "stopped" },
            socket_path().display()
        );
        if let Some(ref r) = llm {
            let st = if r.healthy { "healthy" } else { "unreachable" };
            println!("llm:         {} [{st}] {}", r.provider_id, r.model);
        } else if cfg.llm_configured() && quick {
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
        if let Some(b) = budget {
            println!("budget:      {}", b.message);
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
        println!();
        if ok {
            println!("Result: healthy");
        } else {
            println!("Result: unhealthy");
        }
    }

    if ok { 0 } else { 1 }
}