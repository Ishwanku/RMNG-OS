use crate::audit::{AuditCategory, AuditEntry};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EntityResource {
    pub mcp_calls: u64,
    pub peak_rss_kb_max: u64,
    pub cpu_time_ms_total: u64,
    pub runtime_ms_total: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankedEntityResource {
    pub id: String,
    pub mcp_calls: u64,
    pub peak_rss_kb_max: u64,
    pub cpu_time_ms_total: u64,
    pub runtime_ms_total: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HighResourceCall {
    pub timestamp: String,
    pub agent_id: Option<String>,
    pub mcp_server: Option<String>,
    pub action: String,
    pub peak_rss_kb: Option<u64>,
    pub cpu_time_ms: Option<u64>,
    pub runtime_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRollupReport {
    pub generated_at: String,
    pub total_mcp_calls: u64,
    pub mcp_calls_today: u64,
    pub peak_rss_kb_max: u64,
    pub cpu_time_ms_total: u64,
    pub runtime_ms_total: u64,
    pub by_agent: HashMap<String, EntityResource>,
    pub by_agent_ranked: Vec<RankedEntityResource>,
    pub by_agent_today_ranked: Vec<RankedEntityResource>,
    pub top_consumers: Vec<RankedEntityResource>,
    pub recent_high_resource: Vec<HighResourceCall>,
}

fn add_resource(acc: &mut EntityResource, e: &AuditEntry) {
    acc.mcp_calls += 1;
    if let Some(rss) = e.mcp_peak_rss_kb {
        acc.peak_rss_kb_max = acc.peak_rss_kb_max.max(rss);
    }
    if let Some(cpu) = e.mcp_cpu_time_ms {
        acc.cpu_time_ms_total += cpu;
    }
    if let Some(ms) = e.duration_ms {
        acc.runtime_ms_total += ms;
    }
}

fn rank_entities(map: HashMap<String, EntityResource>) -> Vec<RankedEntityResource> {
    let mut vec: Vec<_> = map.into_iter().collect();
    vec.sort_by(|a, b| {
        b.1.peak_rss_kb_max
            .cmp(&a.1.peak_rss_kb_max)
            .then_with(|| b.1.cpu_time_ms_total.cmp(&a.1.cpu_time_ms_total))
    });
    vec.into_iter()
        .map(|(id, v)| RankedEntityResource {
            id,
            mcp_calls: v.mcp_calls,
            peak_rss_kb_max: v.peak_rss_kb_max,
            cpu_time_ms_total: v.cpu_time_ms_total,
            runtime_ms_total: v.runtime_ms_total,
        })
        .collect()
}

pub fn rollup_mcp_resources(entries: &[AuditEntry]) -> ResourceRollupReport {
    let now = Utc::now();
    let today_key = now.format("%Y-%m-%d").to_string();
    let mut by_agent: HashMap<String, EntityResource> = HashMap::new();
    let mut by_agent_today: HashMap<String, EntityResource> = HashMap::new();
    let mut total_calls = 0u64;
    let mut calls_today = 0u64;
    let mut peak_max = 0u64;
    let mut cpu_total = 0u64;
    let mut runtime_total = 0u64;
    let mut high_calls: Vec<HighResourceCall> = Vec::new();

    for e in entries.iter().filter(|e| e.category == Some(AuditCategory::Mcp)) {
        total_calls += 1;
        if let Some(rss) = e.mcp_peak_rss_kb {
            peak_max = peak_max.max(rss);
        }
        if let Some(cpu) = e.mcp_cpu_time_ms {
            cpu_total += cpu;
        }
        if let Some(ms) = e.duration_ms {
            runtime_total += ms;
        }
        let is_today = e.timestamp.format("%Y-%m-%d").to_string() == today_key;
        if is_today {
            calls_today += 1;
        }
        let agent_key = e.agent_id.clone().unwrap_or_else(|| "-".into());
        add_resource(by_agent.entry(agent_key.clone()).or_default(), e);
        if is_today {
            add_resource(by_agent_today.entry(agent_key).or_default(), e);
        }
        if e.mcp_peak_rss_kb.is_some() || e.mcp_cpu_time_ms.is_some() {
            high_calls.push(HighResourceCall {
                timestamp: e.timestamp.to_rfc3339(),
                agent_id: e.agent_id.clone(),
                mcp_server: e.mcp_server.clone(),
                action: e.action.clone(),
                peak_rss_kb: e.mcp_peak_rss_kb,
                cpu_time_ms: e.mcp_cpu_time_ms,
                runtime_ms: e.duration_ms,
            });
        }
    }

    high_calls.sort_by(|a, b| {
        let sa = a.peak_rss_kb.unwrap_or(0) * 1000 + a.cpu_time_ms.unwrap_or(0);
        let sb = b.peak_rss_kb.unwrap_or(0) * 1000 + b.cpu_time_ms.unwrap_or(0);
        sb.cmp(&sa)
    });
    high_calls.truncate(10);

    let by_agent_ranked = rank_entities(by_agent.clone());
    let by_agent_today_ranked = rank_entities(by_agent_today);
    let top_consumers = by_agent_ranked.iter().take(5).cloned().collect();

    ResourceRollupReport {
        generated_at: now.to_rfc3339(),
        total_mcp_calls: total_calls,
        mcp_calls_today: calls_today,
        peak_rss_kb_max: peak_max,
        cpu_time_ms_total: cpu_total,
        runtime_ms_total: runtime_total,
        by_agent,
        by_agent_ranked,
        by_agent_today_ranked,
        top_consumers,
        recent_high_resource: high_calls,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::AuditEntry;

    #[test]
    fn rolls_up_mcp_resources_by_agent() {
        let mut e1 = AuditEntry::new("mcp.proxy:git.status", "ok");
        e1.category = Some(AuditCategory::Mcp);
        e1.agent_id = Some("repo-keeper".into());
        e1.mcp_peak_rss_kb = Some(4096);
        e1.mcp_cpu_time_ms = Some(100);
        e1.duration_ms = Some(250);
        let mut e2 = AuditEntry::new("mcp.proxy:git.log", "ok");
        e2.category = Some(AuditCategory::Mcp);
        e2.agent_id = Some("repo-keeper".into());
        e2.mcp_peak_rss_kb = Some(8192);
        e2.mcp_cpu_time_ms = Some(50);
        e2.duration_ms = Some(120);
        let r = rollup_mcp_resources(&[e1, e2]);
        assert_eq!(r.total_mcp_calls, 2);
        assert_eq!(r.peak_rss_kb_max, 8192);
        assert_eq!(r.cpu_time_ms_total, 150);
        assert_eq!(r.by_agent_ranked[0].id, "repo-keeper");
        assert_eq!(r.by_agent_ranked[0].peak_rss_kb_max, 8192);
        assert_eq!(r.recent_high_resource.len(), 2);
    }
}
