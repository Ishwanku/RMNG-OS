use rmng_core::{
    budget_governance_report, rollup_llm_costs, rollup_mcp_resources, rollup_recent_days,
    AuditLog, AuditTrack,
    BudgetEnforceMode, IntegrationRegistry, PermissionGate,
    RmngConfig, AUDIT_SCHEMA_VERSION,
};
use rmng_core::SessionStore;
use rmng_nervous::{
    circuit_state_path, health_check_detailed, list_circuit_statuses, load_skill_index,
    reload_from_disk, AgentRegistry, NervousConnector,
};
use chrono::Utc;
use serde::Serialize;

const AUDIT_TAIL: usize = 8;

fn audit_track_label(t: AuditTrack) -> String {
    match t {
        AuditTrack::Native => "native".into(),
        AuditTrack::Mcp => "mcp".into(),
        AuditTrack::Plan => "plan".into(),
    }
}

fn agent_budget_caps() -> Vec<(String, Option<f64>)> {
    AgentRegistry::load()
        .map(|reg| {
            reg.agent_ids()
                .into_iter()
                .filter_map(|id| reg.get(&id).ok().map(|a| (a.id.clone(), a.daily_budget_usd)))
                .collect()
        })
        .unwrap_or_default()
}

#[derive(Serialize)]
struct ObserveCostJson {
    schema_version: u32,
    generated_at: String,
    cost_rollup: rmng_core::CostRollupReport,
    resource_rollup: rmng_core::ResourceRollupReport,
    spent_last_7d_usd: f64,
    budgets: rmng_core::BudgetGovernanceReport,
    circuit_breakers: Vec<rmng_nervous::CircuitStatus>,
    circuit_state_path: String,
    circuits_open: u32,
    rmngd_running: bool,
}

pub async fn print_observe(cost_only: bool, json: bool) {
    reload_from_disk();
    let audit_log = AuditLog::new(AuditLog::default_path());
    let entries = audit_log.read_all().unwrap_or_default();
    let cfg = RmngConfig::load();

    if cost_only || json {
        let rollup = rollup_llm_costs(&entries);
        let resource_rollup = rollup_mcp_resources(&entries);
        let spent_7d = rollup_recent_days(&entries, 7);
        let budgets = budget_governance_report(&cfg, &entries, &agent_budget_caps());
        let circuits = list_circuit_statuses();
        let circuits_open = circuits.iter().filter(|c| c.open).count() as u32;
        if json {
            let out = ObserveCostJson {
                schema_version: 1,
                generated_at: Utc::now().to_rfc3339(),
                cost_rollup: rollup,
                resource_rollup: resource_rollup.clone(),
                spent_last_7d_usd: spent_7d,
                budgets,
                circuit_breakers: circuits,
                circuit_state_path: circuit_state_path().display().to_string(),
                circuits_open,
                rmngd_running: rmng_core::daemon_running(),
            };
            println!("{}", serde_json::to_string_pretty(&out).unwrap_or_default());
            return;
        }
        print_cost_rollups(&rollup, &resource_rollup, spent_7d, &budgets, &circuits);
        if cost_only {
            return;
        }
    }

    println!("=== RMNG observe (v1) ===");
    println!();

    let connector = NervousConnector::from_config(cfg.clone());
    if let Ok(r) = health_check_detailed(connector.config()).await {
        let status = if r.healthy { "healthy" } else { "unreachable" };
        let ep = r.endpoint.as_deref().unwrap_or("-");
        println!(
            "llm health:   {} [{status}] model={} key_set={} endpoint={ep} — {}",
            r.provider_id, r.model, r.api_key_set, r.detail
        );
    }
    if !cfg.llm_fallback.is_empty() {
        println!(
            "llm fallback: global chain → {}",
            cfg.llm_fallback.join(" → ")
        );
    }
    let iso = &cfg.isolation;
    if iso.is_active() {
        println!(
            "isolation:   mem={:?}MB cpu={:?}% pids={:?} cgroup={} session={} no_new_privs={} seccomp={:?} cap_drop={}",
            iso.memory_mb,
            iso.cpu_percent,
            iso.pids_max,
            iso.cgroup,
            iso.new_session,
            iso.no_new_privs,
            iso.seccomp_profile,
            iso.drop_capabilities
        );
    } else {
        println!("isolation:   disabled (set [isolation] in config.toml)");
    }
    print_budget_summary(&cfg, &entries);
    print_resource_summary(&entries);
    reload_from_disk();
    print_circuit_summary();
    println!();

    let daemon_up = rmng_core::daemon_running();
    println!(
        "rmngd:        {}",
        if daemon_up { "running" } else { "stopped" }
    );
    println!("socket:       {}", rmng_core::socket_path().display());
    println!();

    match IntegrationRegistry::load() {
        Ok(reg) => {
            println!("integrations: {} manifests", reg.manifests().len());
            println!("native tools: {} (manifest)", reg.allowed_tool_names().len());
            let handlers = rmng_core::tools::registered_tools();
            println!("handlers:     {} registered", handlers.len());
        }
        Err(e) => println!("integrations: ERROR — {e}"),
    }
    println!();

    let gate = PermissionGate::default();
    match AgentRegistry::load() {
        Ok(agents) => {
            println!("-- agents (multi-level) --");
            for id in agents.agent_ids() {
                if let Ok(a) = agents.get(&id) {
                    let budget_note = a
                        .daily_budget_usd
                        .map(|b| format!(" budget=${b:.2}/d"))
                        .unwrap_or_default();
                    println!(
                        "  {} [{}] — {} native, {} mcp, skills: {}{}",
                        a.id,
                        a.layer,
                        a.allowed_native_tools.len(),
                        a.allowed_mcp_tools.len(),
                        a.skills.join(", "),
                        budget_note
                    );
                }
            }
        }
        Err(e) => println!("agents: ERROR — {e}"),
    }
    println!();

    print_session_observability();
    print_skills_and_mcp(&gate);
    print_audit_tail(&audit_log);
}

fn print_budget_summary(cfg: &RmngConfig, entries: &[rmng_core::AuditEntry]) {
    let report = budget_governance_report(cfg, entries, &agent_budget_caps());
    if cfg.llm_budget.enforce == BudgetEnforceMode::Off && report.global.is_none() {
        if cfg.llm_budget.daily_usd.is_some() {
            println!("llm budget:  configured (enforce=off)");
        }
        return;
    }
    if let Some(ref g) = report.global {
        println!("llm budget:  {:?} — {}", cfg.llm_budget.enforce, g.message);
    }
    if let Some(ref p) = report.active_profile {
        println!("profile budget: {} — {}", p.id, p.check.message);
    }
    for a in report.agents.iter().filter(|a| a.check.level != rmng_core::BudgetLevel::Ok) {
        println!("agent budget: {} — {}", a.id, a.check.message);
    }
}

fn print_circuit_summary() {
    let circuits = list_circuit_statuses();
    let open: Vec<_> = circuits.iter().filter(|c| c.open).collect();
    if open.is_empty() {
        println!(
            "circuits:    {} provider(s) tracked ({})",
            circuits.len(),
            circuit_state_path().display()
        );
    } else {
        println!("circuits:    {} OPEN —", open.len());
        for c in open {
            let rem = c
                .cooldown_secs_remaining
                .map(|s| format!("{s}s remaining"))
                .unwrap_or_else(|| "open".into());
            println!("  {} failures={} {rem}", c.provider_id, c.failures);
        }
    }
}

fn print_session_observability() {
    let session_store = SessionStore::default_store();
    match session_store.list_ids() {
        Ok(ids) => {
            println!("-- sessions ({}) --", session_store.root().display());
            println!("  {} session(s)", ids.len());
            for id in ids.iter().take(5) {
                if let Ok(s) = session_store.load(id) {
                    let mut session_tokens: u64 = 0;
                    let mut session_cost: f64 = 0.0;
                    let mut has_cost = false;
                    let mut by_agent: std::collections::HashMap<String, (u64, f64)> =
                        std::collections::HashMap::new();
                    for call in &s.llm_calls {
                        let tok = call
                            .total_tokens
                            .map(u64::from)
                            .or_else(|| {
                                call.prompt_tokens
                                    .zip(call.completion_tokens)
                                    .map(|(p, c)| (p + c) as u64)
                            })
                            .unwrap_or(0);
                        session_tokens += tok;
                        let cost = call.estimated_cost_usd.unwrap_or(0.0);
                        if call.estimated_cost_usd.is_some() {
                            session_cost += cost;
                            has_cost = true;
                        }
                        let agent = call.agent_id.as_deref().unwrap_or("-").to_string();
                        let e = by_agent.entry(agent).or_insert((0, 0.0));
                        e.0 += tok;
                        e.1 += cost;
                    }
                    let cost_line = if has_cost {
                        format!(" est_cost=${session_cost:.4}")
                    } else {
                        String::new()
                    };
                    println!(
                        "  {} — handoffs: {}, llm calls: {}, tokens={}{}",
                        id,
                        s.handoff_history.len(),
                        s.llm_calls.len(),
                        session_tokens,
                        cost_line
                    );
                    let mut agents: Vec<_> = by_agent.into_iter().collect();
                    agents.sort_by(|a, b| b.1.1.partial_cmp(&a.1.1).unwrap_or(std::cmp::Ordering::Equal));
                    for (agent, (tok, cost)) in agents.iter().take(3) {
                        println!("      {agent}: tokens={tok} cost=${cost:.4}");
                    }
                }
            }
        }
        Err(e) => println!("sessions: ERROR — {e}"),
    }
    println!();
}

fn print_skills_and_mcp(gate: &PermissionGate) {
    match load_skill_index() {
        Ok(index) => println!("skills:      {} indexed", index.len()),
        Err(e) => println!("skills:      ERROR — {e}"),
    }
    let allowlist = gate.mcp_allowlist();
    let n = allowlist.servers.len();
    let enabled = allowlist.servers.values().filter(|c| c.enabled).count();
    println!("mcp servers: {n} configured, {enabled} enabled");
    println!();
}

fn print_audit_tail(audit_log: &AuditLog) {
    let audit_path = audit_log.path();
    println!("-- audit chain (schema v{AUDIT_SCHEMA_VERSION}) --");
    match audit_log.verify_chain() {
        Ok(v) => println!(
            "  {} entries — {}",
            v.entries,
            if v.valid { "chain valid" } else { "TAMPER DETECTED" }
        ),
        Err(e) => println!("  verify error: {e}"),
    }
    println!();
    println!("-- recent audit ({}) --", audit_path.display());
    match audit_log.tail(AUDIT_TAIL) {
        Ok(entries) if entries.is_empty() => println!("  (no entries)"),
        Ok(entries) => {
            for e in entries {
                let track = e.track.map(audit_track_label).unwrap_or_else(|| "-".into());
                let cat = e.category.map(|c| c.as_str()).unwrap_or("-");
                let cost = e.cost_usd.map(|c| format!(" ${c:.6}")).unwrap_or_default();
                let agent = e.agent_id.as_deref().unwrap_or("-");
                let rss = e
                    .mcp_peak_rss_kb
                    .map(|r| format!(" rss={r}KB"))
                    .unwrap_or_default();
                println!(
                    "  [{}] {cat} {agent} {} {}{}{rss}",
                    e.timestamp.format("%H:%M:%S"),
                    e.action,
                    track,
                    cost
                );
            }
        }
        Err(e) => println!("  ERROR: {e}"),
    }
}



fn print_resource_summary(entries: &[rmng_core::AuditEntry]) {
    let rollup = rollup_mcp_resources(entries);
    if rollup.total_mcp_calls == 0 {
        return;
    }
    println!(
        "mcp resources: {} calls, peak_rss_max={}KB cpu_total={}ms runtime_total={}ms",
        rollup.total_mcp_calls,
        rollup.peak_rss_kb_max,
        rollup.cpu_time_ms_total,
        rollup.runtime_ms_total
    );
    if !rollup.top_consumers.is_empty() {
        println!("  top consumers (peak RSS):");
        for c in &rollup.top_consumers {
            println!(
                "    {}  peak={}KB cpu={}ms calls={}",
                c.id, c.peak_rss_kb_max, c.cpu_time_ms_total, c.mcp_calls
            );
        }
    }
    if !rollup.recent_high_resource.is_empty() {
        println!("  recent high-resource MCP calls:");
        for h in rollup.recent_high_resource.iter().take(3) {
            let agent = h.agent_id.as_deref().unwrap_or("-");
            println!(
                "    {} {} peak={:?}KB cpu={:?}ms",
                h.timestamp, agent, h.peak_rss_kb, h.cpu_time_ms
            );
        }
    }
}

fn print_cost_rollups(
    rollup: &rmng_core::CostRollupReport,
    resources: &rmng_core::ResourceRollupReport,
    spent_7d: f64,
    budgets: &rmng_core::BudgetGovernanceReport,
    circuits: &[rmng_nervous::CircuitStatus],
) {
    println!("=== RMNG observe --cost ===");
    println!();
    println!(
        "total:       ${:.4} ({} LLM calls)",
        rollup.total_cost_usd, rollup.total_llm_calls
    );
    println!(
        "today:       ${:.4} ({} calls)",
        rollup.spent_today_usd, rollup.llm_calls_today
    );
    println!("last 7d:     ${spent_7d:.4}");
    if let Some(ref g) = budgets.global {
        println!("budget:      {}", g.message);
    }
    if let Some(ref p) = budgets.active_profile {
        println!("profile:     {} — {}", p.id, p.check.message);
    }
    if !rollup.daily.is_empty() {
        println!();
        println!("-- daily --");
        for d in &rollup.daily {
            println!("  {}  ${:.4}  {} calls", d.period, d.cost_usd, d.llm_calls);
        }
    }
    if !rollup.by_agent_today_ranked.is_empty() {
        println!();
        println!("-- agents today --");
        for v in rollup.by_agent_today_ranked.iter().take(10) {
            let budget = budgets.agents.iter().find(|a| a.id == v.id);
            let bmsg = budget
                .map(|b| format!(" [{:?}]", b.check.level))
                .unwrap_or_default();
            println!(
                "  {}  ${:.4}  {} calls  tok={}+{}{}",
                v.id, v.cost_usd, v.llm_calls, v.tokens_prompt, v.tokens_completion, bmsg
            );
        }
    }
    if !rollup.by_session_today_ranked.is_empty() {
        println!();
        println!("-- sessions today --");
        for v in rollup.by_session_today_ranked.iter().take(10) {
            println!(
                "  {}  ${:.4}  {} calls",
                v.id, v.cost_usd, v.llm_calls
            );
        }
    }
    if !rollup.by_agent_ranked.is_empty() {
        println!();
        println!("-- agents (all time) --");
        for v in rollup.by_agent_ranked.iter().take(10) {
            println!("  {}  ${:.4}  {} calls", v.id, v.cost_usd, v.llm_calls);
        }
    }
    let open: Vec<_> = circuits.iter().filter(|c| c.open).collect();
    println!();
    println!(
        "circuits:    {} tracked, {} open ({})",
        circuits.len(),
        open.len(),
        circuit_state_path().display()
    );
    for c in open {
        println!("  OPEN {} failures={}", c.provider_id, c.failures);
    }
    if resources.total_mcp_calls > 0 {
        println!();
        println!("-- MCP resources --");
        println!(
            "  calls: {} today={}, peak_rss_max={}KB, cpu_total={}ms",
            resources.total_mcp_calls,
            resources.mcp_calls_today,
            resources.peak_rss_kb_max,
            resources.cpu_time_ms_total
        );
        if !resources.by_agent_today_ranked.is_empty() {
            println!("  agents today (by peak RSS):");
            for a in resources.by_agent_today_ranked.iter().take(5) {
                println!(
                    "    {}  peak={}KB cpu={}ms calls={}",
                    a.id, a.peak_rss_kb_max, a.cpu_time_ms_total, a.mcp_calls
                );
            }
        }
        if !resources.recent_high_resource.is_empty() {
            println!("  recent high-resource:");
            for h in resources.recent_high_resource.iter().take(5) {
                let agent = h.agent_id.as_deref().unwrap_or("-");
                println!(
                    "    {} {} peak={:?}KB cpu={:?}ms",
                    &h.timestamp[..19.min(h.timestamp.len())],
                    agent,
                    h.peak_rss_kb,
                    h.cpu_time_ms
                );
            }
        }
    }
}
