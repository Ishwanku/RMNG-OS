use crate::audit::{AuditCategory, AuditEntry};
use chrono::{DateTime, Datelike, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EntityCost {
    pub cost_usd: f64,
    pub llm_calls: u64,
    pub tokens_prompt: u64,
    pub tokens_completion: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeriodCost {
    pub period: String,
    pub cost_usd: f64,
    pub llm_calls: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostRollupReport {
    pub generated_at: String,
    pub total_cost_usd: f64,
    pub total_llm_calls: u64,
    pub by_session: HashMap<String, EntityCost>,
    pub by_agent: HashMap<String, EntityCost>,
    pub daily: Vec<PeriodCost>,
    pub weekly: Vec<PeriodCost>,
}

fn add_cost(acc: &mut EntityCost, e: &AuditEntry) {
    acc.llm_calls += 1;
    if let Some(c) = e.cost_usd {
        acc.cost_usd += c;
    }
    if let Some(p) = e.tokens_prompt {
        acc.tokens_prompt += p as u64;
    }
    if let Some(c) = e.tokens_completion {
        acc.tokens_completion += c as u64;
    }
}

fn week_key(dt: DateTime<Utc>) -> String {
    let iso = dt.iso_week();
    format!("{}-W{:02}", iso.year(), iso.week())
}

pub fn rollup_llm_costs(entries: &[AuditEntry]) -> CostRollupReport {
    let now = Utc::now();
    let mut by_session: HashMap<String, EntityCost> = HashMap::new();
    let mut by_agent: HashMap<String, EntityCost> = HashMap::new();
    let mut daily_map: HashMap<String, EntityCost> = HashMap::new();
    let mut weekly_map: HashMap<String, EntityCost> = HashMap::new();
    let mut total_cost = 0.0f64;
    let mut total_calls = 0u64;

    for e in entries.iter().filter(|e| e.category == Some(AuditCategory::Llm)) {
        total_calls += 1;
        if let Some(c) = e.cost_usd {
            total_cost += c;
        }
        if let Some(sid) = &e.session_id {
            add_cost(by_session.entry(sid.clone()).or_default(), e);
        }
        if let Some(aid) = &e.agent_id {
            add_cost(by_agent.entry(aid.clone()).or_default(), e);
        }
        let day = e.timestamp.format("%Y-%m-%d").to_string();
        add_cost(daily_map.entry(day).or_default(), e);
        let week = week_key(e.timestamp);
        add_cost(weekly_map.entry(week).or_default(), e);
    }

    let mut daily: Vec<PeriodCost> = daily_map
        .into_iter()
        .map(|(period, v)| PeriodCost {
            period,
            cost_usd: v.cost_usd,
            llm_calls: v.llm_calls,
        })
        .collect();
    daily.sort_by(|a, b| b.period.cmp(&a.period));
    daily.truncate(14);

    let mut weekly: Vec<PeriodCost> = weekly_map
        .into_iter()
        .map(|(period, v)| PeriodCost {
            period,
            cost_usd: v.cost_usd,
            llm_calls: v.llm_calls,
        })
        .collect();
    weekly.sort_by(|a, b| b.period.cmp(&a.period));
    weekly.truncate(8);

    // Sort entity maps by cost descending for display
    let mut session_vec: Vec<_> = by_session.into_iter().collect();
    session_vec.sort_by(|a, b| b.1.cost_usd.partial_cmp(&a.1.cost_usd).unwrap_or(std::cmp::Ordering::Equal));
    let by_session: HashMap<_, _> = session_vec.into_iter().collect();

    let mut agent_vec: Vec<_> = by_agent.into_iter().collect();
    agent_vec.sort_by(|a, b| b.1.cost_usd.partial_cmp(&a.1.cost_usd).unwrap_or(std::cmp::Ordering::Equal));
    let by_agent: HashMap<_, _> = agent_vec.into_iter().collect();

    CostRollupReport {
        generated_at: now.to_rfc3339(),
        total_cost_usd: total_cost,
        total_llm_calls: total_calls,
        by_session,
        by_agent,
        daily,
        weekly,
    }
}

pub fn rollup_recent_days(entries: &[AuditEntry], days: i64) -> f64 {
    let cutoff = Utc::now() - Duration::days(days);
    entries
        .iter()
        .filter(|e| e.category == Some(AuditCategory::Llm))
        .filter(|e| e.timestamp >= cutoff)
        .filter_map(|e| e.cost_usd)
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::AuditEntry;

    #[test]
    fn rolls_up_by_session_and_agent() {
        let mut e1 = AuditEntry::new("nervous.llm_call", "success");
        e1.category = Some(AuditCategory::Llm);
        e1.session_id = Some("s1".into());
        e1.agent_id = Some("repo-keeper".into());
        e1.cost_usd = Some(0.1);
        let mut e2 = AuditEntry::new("nervous.llm_call", "success");
        e2.category = Some(AuditCategory::Llm);
        e2.session_id = Some("s1".into());
        e2.agent_id = Some("repo-keeper".into());
        e2.cost_usd = Some(0.2);
        let r = rollup_llm_costs(&[e1, e2]);
        assert!((r.total_cost_usd - 0.3).abs() < 0.001);
        assert_eq!(r.by_session.get("s1").unwrap().llm_calls, 2);
        assert_eq!(r.by_agent.get("repo-keeper").unwrap().llm_calls, 2);
    }
}