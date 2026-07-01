use crate::audit::{AuditCategory, AuditEntry, AuditLog};
use crate::config::{BudgetEnforceMode, RmngConfig};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BudgetLevel {
    Ok,
    Warn,
    Deny,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct BudgetCheckResult {
    pub level: BudgetLevel,
    pub allowed: bool,
    pub spent_today_usd: f64,
    pub budget_usd: f64,
    pub warn_at_usd: f64,
    pub deny_at_usd: f64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ScopedBudgetStatus {
    pub id: String,
    pub check: BudgetCheckResult,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct BudgetGovernanceReport {
    pub global: Option<BudgetCheckResult>,
    pub active_profile: Option<ScopedBudgetStatus>,
    pub profiles: Vec<ScopedBudgetStatus>,
    pub agents: Vec<ScopedBudgetStatus>,
}

/// Sum LLM costs from audit log for UTC calendar day containing `now`.
pub fn spent_today_usd(entries: &[AuditEntry], now: DateTime<Utc>) -> f64 {
    spent_today_for_agent(entries, now, None)
}

/// Sum today's LLM cost, optionally scoped to one agent_id.
pub fn spent_today_for_agent(
    entries: &[AuditEntry],
    now: DateTime<Utc>,
    agent_id: Option<&str>,
) -> f64 {
    let day = now.format("%Y-%m-%d").to_string();
    entries
        .iter()
        .filter(|e| e.category == Some(AuditCategory::Llm))
        .filter(|e| e.timestamp.format("%Y-%m-%d").to_string() == day)
        .filter(|e| match agent_id {
            Some(aid) => e.agent_id.as_deref() == Some(aid),
            None => true,
        })
        .filter_map(|e| e.cost_usd)
        .sum()
}

/// Sum today's LLM cost for a named profile (Sprint 19).
pub fn spent_today_for_profile(
    entries: &[AuditEntry],
    now: DateTime<Utc>,
    profile: &str,
) -> f64 {
    let day = now.format("%Y-%m-%d").to_string();
    entries
        .iter()
        .filter(|e| e.category == Some(AuditCategory::Llm))
        .filter(|e| e.timestamp.format("%Y-%m-%d").to_string() == day)
        .filter(|e| e.llm_profile.as_deref() == Some(profile))
        .filter_map(|e| e.cost_usd)
        .sum()
}

fn resolve_daily_cap(cfg: &RmngConfig, profile_cap: Option<f64>, agent_cap: Option<f64>) -> Option<f64> {
    agent_cap.or(profile_cap).or(cfg.llm_budget.daily_usd)
}

fn resolve_profile_cap(cfg: &RmngConfig, profile_name: &str) -> Option<f64> {
    cfg.profiles
        .iter()
        .find(|p| p.name == profile_name)
        .and_then(|p| p.daily_budget_usd)
}

fn build_check(
    cfg: &RmngConfig,
    spent: f64,
    daily: f64,
    scope: Option<String>,
) -> BudgetCheckResult {
    let budget = &cfg.llm_budget;
    let warn_frac = budget.warn_threshold.unwrap_or(0.8);
    let deny_frac = budget.deny_threshold.unwrap_or(1.0);
    let warn_at = daily * warn_frac;
    let deny_at = daily * deny_frac;

    let level = if spent >= deny_at {
        BudgetLevel::Deny
    } else if spent >= warn_at {
        BudgetLevel::Warn
    } else {
        BudgetLevel::Ok
    };

    let allowed = match budget.enforce {
        BudgetEnforceMode::Deny => level != BudgetLevel::Deny,
        BudgetEnforceMode::Warn | BudgetEnforceMode::Off => true,
    };

    let scope_suffix = scope
        .as_ref()
        .map(|s| format!(" ({s})"))
        .unwrap_or_default();
    let message = match level {
        BudgetLevel::Ok => format!("spent ${spent:.4} / ${daily:.2} today{scope_suffix}"),
        BudgetLevel::Warn => format!(
            "budget warn: spent ${spent:.4} >= ${warn_at:.2} (cap ${daily:.2}){scope_suffix}"
        ),
        BudgetLevel::Deny => format!(
            "budget deny: spent ${spent:.4} >= ${deny_at:.2} (cap ${daily:.2}){scope_suffix}"
        ),
    };

    BudgetCheckResult {
        level,
        allowed,
        spent_today_usd: spent,
        budget_usd: daily,
        warn_at_usd: warn_at,
        deny_at_usd: deny_at,
        message,
        scope,
    }
}

pub fn check_budget(cfg: &RmngConfig, entries: &[AuditEntry]) -> Option<BudgetCheckResult> {
    check_budget_for_agent(cfg, entries, None, None, None)
}

pub fn check_budget_for_agent(
    cfg: &RmngConfig,
    entries: &[AuditEntry],
    agent_id: Option<&str>,
    profile_name: Option<&str>,
    agent_cap: Option<f64>,
) -> Option<BudgetCheckResult> {
    let budget = &cfg.llm_budget;
    if budget.enforce == BudgetEnforceMode::Off && budget.daily_usd.is_none() && agent_cap.is_none() {
        if profile_name.is_none() {
            return None;
        }
    }
    let profile_cap = profile_name.and_then(|n| resolve_profile_cap(cfg, n));
    let daily = resolve_daily_cap(cfg, profile_cap, agent_cap)?;
    if daily <= 0.0 {
        return None;
    }
    let now = Utc::now();
    let spent = if let Some(aid) = agent_id {
        spent_today_for_agent(entries, now, Some(aid))
    } else if let Some(prof) = profile_name {
        spent_today_for_profile(entries, now, prof)
    } else {
        spent_today_usd(entries, now)
    };
    let scope = agent_id
        .map(|a| format!("agent={a}"))
        .or_else(|| profile_name.map(|p| format!("profile={p}")));
    Some(build_check(cfg, spent, daily, scope))
}

pub fn check_budget_from_audit(cfg: &RmngConfig) -> Option<BudgetCheckResult> {
    check_budget_from_audit_for_agent(cfg, None, None)
}

pub fn check_budget_from_audit_for_agent(
    cfg: &RmngConfig,
    agent_id: Option<&str>,
    agent_cap: Option<f64>,
) -> Option<BudgetCheckResult> {
    let log = AuditLog::new(AuditLog::default_path());
    let entries = log.read_all().ok()?;
    check_budget_for_agent(cfg, &entries, agent_id, None, agent_cap)
}

/// Full budget picture for observe / health JSON (Sprint 19).
pub fn budget_governance_report(
    cfg: &RmngConfig,
    entries: &[AuditEntry],
    agent_caps: &[(String, Option<f64>)],
) -> BudgetGovernanceReport {
    let mut report = BudgetGovernanceReport::default();
    if cfg.llm_budget.daily_usd.is_some() || cfg.llm_budget.enforce != BudgetEnforceMode::Off {
        report.global = check_budget_for_agent(cfg, entries, None, None, None);
    }
    if let Some(ref active) = cfg.profile {
        if let Some(cap) = resolve_profile_cap(cfg, active) {
            if let Some(check) = check_budget_for_agent(cfg, entries, None, Some(active), Some(cap)) {
                report.active_profile = Some(ScopedBudgetStatus {
                    id: active.clone(),
                    check,
                });
            }
        }
    }
    for p in &cfg.profiles {
        if let Some(cap) = p.daily_budget_usd {
            if let Some(check) =
                check_budget_for_agent(cfg, entries, None, Some(&p.name), Some(cap))
            {
                report.profiles.push(ScopedBudgetStatus {
                    id: p.name.clone(),
                    check,
                });
            }
        }
    }
    for (agent_id, cap) in agent_caps {
        if let Some(check) =
            check_budget_for_agent(cfg, entries, Some(agent_id), None, *cap)
        {
            report.agents.push(ScopedBudgetStatus {
                id: agent_id.clone(),
                check,
            });
        }
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::AuditEntry;

    #[test]
    fn sums_llm_cost_for_today() {
        let today = Utc::now();
        let mut e1 = AuditEntry::new("nervous.llm_call", "success");
        e1.category = Some(AuditCategory::Llm);
        e1.timestamp = today;
        e1.cost_usd = Some(0.5);
        let mut e2 = AuditEntry::new("nervous.llm_call", "success");
        e2.category = Some(AuditCategory::Llm);
        e2.timestamp = today;
        e2.cost_usd = Some(0.25);
        let yesterday = today - chrono::Duration::days(1);
        let mut old = AuditEntry::new("nervous.llm_call", "success");
        old.category = Some(AuditCategory::Llm);
        old.timestamp = yesterday;
        old.cost_usd = Some(99.0);
        assert!((spent_today_usd(&[e1, e2, old], today) - 0.75).abs() < 0.001);
    }

    #[test]
    fn deny_when_over_threshold() {
        let mut cfg = RmngConfig::default();
        cfg.llm_budget.daily_usd = Some(1.0);
        cfg.llm_budget.warn_threshold = Some(0.5);
        cfg.llm_budget.deny_threshold = Some(1.0);
        cfg.llm_budget.enforce = BudgetEnforceMode::Deny;
        let today = Utc::now();
        let mut e = AuditEntry::new("nervous.llm_call", "success");
        e.category = Some(AuditCategory::Llm);
        e.timestamp = today;
        e.cost_usd = Some(1.2);
        let r = check_budget(&cfg, &[e]).unwrap();
        assert_eq!(r.level, BudgetLevel::Deny);
        assert!(!r.allowed);
    }

    #[test]
    fn profile_budget_scopes_spend() {
        let mut cfg = RmngConfig::default();
        cfg.profile = Some("fast".into());
        cfg.profiles = vec![crate::config::LlmProfile {
            name: "fast".into(),
            daily_budget_usd: Some(2.0),
            ..Default::default()
        }];
        cfg.llm_budget.enforce = BudgetEnforceMode::Warn;
        let today = Utc::now();
        let mut e = AuditEntry::new("nervous.llm_call", "success");
        e.category = Some(AuditCategory::Llm);
        e.timestamp = today;
        e.llm_profile = Some("fast".into());
        e.cost_usd = Some(1.7);
        let r = check_budget_for_agent(&cfg, &[e], None, Some("fast"), Some(2.0)).unwrap();
        assert_eq!(r.level, BudgetLevel::Warn);
    }
}
