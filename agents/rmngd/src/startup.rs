//! Startup validation and logging (Sprint 28).

use rmng_core::{agent_registry_check, CheckLevel, ReadinessReport};
use rmng_nervous::AgentRegistry;
use tracing::{error, info, warn};

pub fn full_readiness() -> ReadinessReport {
    let mut report = ReadinessReport::run();
    let agent_result = AgentRegistry::load()
        .map(|r| r.agent_ids().len())
        .map_err(|e| e.to_string());
    report.push_check(agent_registry_check(Some(agent_result)));
    report
}

pub fn log_readiness(report: &ReadinessReport) {
    for check in &report.checks {
        match check.level {
            CheckLevel::Ok => info!(check = %check.id, "{}", check.message),
            CheckLevel::Warn => warn!(check = %check.id, "{}", check.message),
            CheckLevel::Error => error!(check = %check.id, "{}", check.message),
        }
    }
    if report.ok {
        info!("rmngd startup validation passed");
    } else {
        error!("rmngd startup validation failed — starting in degraded mode");
    }
}

pub fn print_validate_human(report: &ReadinessReport) {
    println!("=== rmngd --validate ===");
    for check in &report.checks {
        let tag = match check.level {
            CheckLevel::Ok => "OK",
            CheckLevel::Warn => "WARN",
            CheckLevel::Error => "ERROR",
        };
        println!("  [{tag}] {} — {}", check.id, check.message);
    }
    println!();
    if report.ok {
        println!("Result: ready");
    } else {
        println!("Result: not ready (fix ERROR items before production)");
    }
}