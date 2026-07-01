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
}

/// Sum LLM costs from audit log for UTC calendar day containing `now`.
pub fn spent_today_usd(entries: &[AuditEntry], now: DateTime<Utc>) -> f64 {
    let day = now.format("%Y-%m-%d").to_string();
    entries
        .iter()
        .filter(|e| e.category == Some(AuditCategory::Llm))
        .filter(|e| e.timestamp.format("%Y-%m-%d").to_string() == day)
        .filter_map(|e| e.cost_usd)
        .sum()
}

pub fn check_budget(cfg: &RmngConfig, entries: &[AuditEntry]) -> Option<BudgetCheckResult> {
    let budget = &cfg.llm_budget;
    if budget.enforce == BudgetEnforceMode::Off {
        return None;
    }
    let daily = budget.daily_usd?;
    if daily <= 0.0 {
        return None;
    }
    let warn_frac = budget.warn_threshold.unwrap_or(0.8);
    let deny_frac = budget.deny_threshold.unwrap_or(1.0);
    let warn_at = daily * warn_frac;
    let deny_at = daily * deny_frac;
    let spent = spent_today_usd(entries, Utc::now());

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

    let message = match level {
        BudgetLevel::Ok => format!("spent ${spent:.4} / ${daily:.2} today"),
        BudgetLevel::Warn => format!("budget warn: spent ${spent:.4} >= ${warn_at:.2} (cap ${daily:.2})"),
        BudgetLevel::Deny => format!("budget deny: spent ${spent:.4} >= ${deny_at:.2} (cap ${daily:.2})"),
    };

    Some(BudgetCheckResult {
        level,
        allowed,
        spent_today_usd: spent,
        budget_usd: daily,
        warn_at_usd: warn_at,
        deny_at_usd: deny_at,
        message,
    })
}

pub fn check_budget_from_audit(cfg: &RmngConfig) -> Option<BudgetCheckResult> {
    let log = AuditLog::new(AuditLog::default_path());
    let entries = log.read_all().ok()?;
    check_budget(cfg, &entries)
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
}