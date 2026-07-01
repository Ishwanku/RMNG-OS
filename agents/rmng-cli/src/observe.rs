use rmng_core::{
    check_budget_from_audit, rollup_llm_costs, rollup_recent_days, AuditCategory, AuditLog,
    AuditTrack, BudgetEnforceMode, IntegrationRegistry, PermissionGate, RmngConfig,
    AUDIT_SCHEMA_VERSION,
};

fn audit_track_label(t: AuditTrack) -> String {
    match t {
        AuditTrack::Native => "native".into(),
        AuditTrack::Mcp => "mcp".into(),
        AuditTrack::Plan => "plan".into(),
    }
}
use rmng_core::SessionStore;
use rmng_nervous::{
    circuit_state_path, health_check_detailed, list_circuit_statuses, load_skill_index,
    reload_from_disk, AgentRegistry, NervousConnector,
};
use serde::Serialize;

const AUDIT_TAIL: usize = 8;

#[derive(Serialize)]
struct ObserveCostJson {
    cost_rollup: rmng_core::CostRollupReport,
    spent_last_7d_usd: f64,
    budget: Option<rmng_core::BudgetCheckResult>,
    circuit_breakers: Vec<rmng_nervous::CircuitStatus>,
}

pub async fn print_observe(cost_only: bool, json: bool) {
    reload_from_disk();
    let audit_log = AuditLog::new(AuditLog::default_path());
    let entries = audit_log.read_all().unwrap_or_default();

    if cost_only || json {
        let rollup = rollup_llm_costs(&entries);
        let spent_7d = rollup_recent_days(&entries, 7);
        let budget = check_budget_from_audit(&RmngConfig::load());
        let circuits = list_circuit_statuses();
        if json {
            let out = ObserveCostJson {
                cost_rollup: rollup,
                spent_last_7d_usd: spent_7d,
                budget,
                circuit_breakers: circuits,
            };
            println!("{}", serde_json::to_string_pretty(&out).unwrap_or_default());
            return;
        }
        print_cost_rollups(&rollup, spent_7d, budget.as_ref(), &circuits);
        if cost_only {
            return;
        }
    }

    println!("=== RMNG observe ===");
    println!();

    let connector = NervousConnector::load();
    let cfg = connector.config();
    println!(
        "llm:          {} ({})",
        connector.provider_label(),
        rmng_core::RmngConfig::config_path().display()
    );
    if let Ok(r) = health_check_detailed(cfg).await {
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
            "isolation:   mem={:?}MB cpu={:?}% pids={:?} cgroup={} session={} no_new_privs={}",
            iso.memory_mb,
            iso.cpu_percent,
            iso.pids_max,
            iso.cgroup,
            iso.new_session,
            iso.no_new_privs
        );
    } else {
        println!("isolation:   disabled (set [isolation] in config.toml)");
    }
    let budget_cfg = &cfg.llm_budget;
    if budget_cfg.enforce != BudgetEnforceMode::Off {
        if let Some(b) = check_budget_from_audit(&cfg) {
            println!(
                "llm budget:  {:?} — {}",
                budget_cfg.enforce,
                b.message
            );
        }
    } else if budget_cfg.daily_usd.is_some() {
        println!("llm budget:  configured (enforce=off)");
    }
    reload_from_disk();
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
            println!(
                "  {} failures={} {rem}",
                c.provider_id, c.failures
            );
        }
    }
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
            println!();
            println!("-- manifest tools --");
            for t in reg.allowed_tool_names() {
                let tag = if handlers.contains(&t.as_str()) {
                    "handler"
                } else {
                    "manifest-only"
                };
                println!("  {t} ({tag})");
            }
        }
        Err(e) => println!("integrations: ERROR — {e}"),
    }
    println!();

    let gate = PermissionGate::default();
    println!("-- permission gate (native) --");
    for t in gate.allowed_tools() {
        println!("  {t}");
    }
    println!();

    match AgentRegistry::load() {
        Ok(agents) => {
            println!("-- agents (multi-level) --");
            for id in agents.agent_ids() {
                if let Ok(a) = agents.get(&id) {
                    println!(
                        "  {} [{}] — {} native, {} mcp, skills: {}",
                        a.id,
                        a.layer,
                        a.allowed_native_tools.len(),
                        a.allowed_mcp_tools.len(),
                        a.skills.join(", ")
                    );
                }
            }
        }
        Err(e) => println!("agents: ERROR — {e}"),
    }
    println!();

    let session_store = SessionStore::default_store();
    match session_store.list_ids() {
        Ok(ids) => {
            println!("-- sessions store ({}) --", session_store.root().display());
            println!("  {} session(s)", ids.len());
            for id in ids.iter().take(5) {
                if let Ok(s) = session_store.load(id) {
                    println!(
                        "  {} — handoffs: {}, active layers: {}, llm calls: {}",
                        id,
                        s.handoff_history.len(),
                        s.active_agents.len(),
                        s.llm_calls.len()
                    );
                    let mut session_tokens: u64 = 0;
                    let mut session_cost: f64 = 0.0;
                    let mut has_cost = false;
                    for call in &s.llm_calls {
                        if let Some(t) = call.total_tokens {
                            session_tokens += t as u64;
                        } else if let (Some(p), Some(c)) = (call.prompt_tokens, call.completion_tokens) {
                            session_tokens += (p + c) as u64;
                        }
                        if let Some(c) = call.estimated_cost_usd {
                            session_cost += c;
                            has_cost = true;
                        }
                    }
                    if !s.llm_calls.is_empty() {
                        let cost_line = if has_cost {
                            format!(" est_cost=${session_cost:.4}")
                        } else {
                            String::new()
                        };
                        println!(
                            "      Σ session tokens={session_tokens}{cost_line} ({} calls)",
                            s.llm_calls.len()
                        );
                    }
                    for call in s.llm_calls.iter().rev().take(3) {
                        let agent = call.agent_id.as_deref().unwrap_or("-");
                        let tokens = match (call.prompt_tokens, call.completion_tokens, call.total_tokens) {
                            (Some(p), Some(c), _) => format!("tokens={p}+{c}"),
                            (_, _, Some(t)) => format!("tokens={t}"),
                            _ => "tokens=-".into(),
                        };
                        let cost = call
                            .estimated_cost_usd
                            .map(|c| format!(" cost=${c:.6}"))
                            .unwrap_or_default();
                        let fb = if call.fallback_index > 0 {
                            format!(" fallback#{}", call.fallback_index)
                        } else {
                            String::new()
                        };
                        println!(
                            "      {} {} [{}] {} {} {}ms{fb}",
                            call.timestamp.format("%H:%M:%S"),
                            agent,
                            call.profile_label,
                            call.provider,
                            call.model,
                            call.latency_ms
                        );
                        println!("        {tokens}{cost}");
                    }
                }
            }
        }
        Err(e) => println!("sessions: ERROR — {e}"),
    }

    println!();

    match load_skill_index() {
        Ok(index) => {
            println!("-- skills index ({} loaded, body on demand) --", index.len());
            for s in index.iter().take(10) {
                let desc = if s.description.len() > 60 {
                    format!("{}…", &s.description[..60])
                } else {
                    s.description.clone()
                };
                println!("  {} — {desc}", s.name);
            }
            if index.len() > 10 {
                println!("  … and {} more", index.len() - 10);
            }
        }
        Err(e) => println!("skills index: ERROR — {e}"),
    }
    println!();

    let allowlist = gate.mcp_allowlist();
    println!("-- mcp allowlist (ephemeral per-call subprocess) --");
    if allowlist.servers.is_empty() {
        println!("  (none configured)");
    } else {
        for (name, cfg) in &allowlist.servers {
            let state = if cfg.enabled { "enabled" } else { "disabled" };
            let iso = cfg
                .isolation
                .as_ref()
                .filter(|i| i.is_active())
                .map(|i| format!(" iso:mem={:?}MB", i.memory_mb))
                .unwrap_or_default();
            println!(
                "  {name} [{state}] {} — tools: {}{iso}",
                cfg.command,
                cfg.allowed_tools.join(", ")
            );
        }
    }
    println!("  note: MCP children are spawned per request with optional cgroup/rlimit isolation");
    println!();

    let cfg = RmngConfig::load();
    let _ = cfg;

    let audit_path = audit_log.path();
    println!("-- audit chain (schema v{AUDIT_SCHEMA_VERSION}) --");
    match audit_log.verify_chain() {
        Ok(v) => println!(
            "  {} entries — {}",
            v.entries,
            if v.valid { "✓ chain valid" } else { "✗ TAMPER DETECTED" }
        ),
        Err(e) => println!("  verify error: {e}"),
    }
    println!();

    println!("-- recent audit ({}) --", audit_path.display());
    match audit_log.tail(AUDIT_TAIL) {
        Ok(entries) if entries.is_empty() => println!("  (no entries)"),
        Ok(entries) => {
            let mut llm_cost = 0.0f64;
            let mut llm_calls = 0u32;
            let mut mcp_spawns = 0u32;
            for e in &entries {
                if e.category == Some(AuditCategory::Llm) {
                    llm_calls += 1;
                    if let Some(c) = e.cost_usd {
                        llm_cost += c;
                    }
                }
                if e.category == Some(AuditCategory::Mcp) {
                    mcp_spawns += 1;
                }
            }
            if llm_calls > 0 || mcp_spawns > 0 {
                println!(
                    "  tail stats: llm_calls={llm_calls} est_cost=${llm_cost:.4} mcp_calls={mcp_spawns}"
                );
            }
            for e in entries {
                let track = e
                    .track
                    .map(audit_track_label)
                    .unwrap_or_else(|| "-".into());
                let cat = e
                    .category
                    .map(|c| c.as_str())
                    .unwrap_or("-");
                let dur = e
                    .duration_ms
                    .map(|d| format!("{d}ms"))
                    .unwrap_or_else(|| "-".into());
                let seq = if e.seq > 0 {
                    format!("#{}", e.seq)
                } else {
                    "-".into()
                };
                let cost = e
                    .cost_usd
                    .map(|c| format!(" ${c:.6}"))
                    .unwrap_or_default();
                println!(
                    "  [{seq}] [{}] {cat} {} {} {track} {dur}{cost} — {}",
                    e.timestamp.format("%H:%M:%S"),
                    e.outcome,
                    e.action,
                    e.detail.as_deref().unwrap_or("")
                );
            }
        }
        Err(e) => println!("  ERROR: {e}"),
    }
}

fn print_cost_rollups(
    rollup: &rmng_core::CostRollupReport,
    spent_7d: f64,
    budget: Option<&rmng_core::BudgetCheckResult>,
    circuits: &[rmng_nervous::CircuitStatus],
) {
    println!("=== RMNG observe --cost ===");
    println!();
    println!(
        "total:       ${:.4} ({} LLM calls)",
        rollup.total_cost_usd, rollup.total_llm_calls
    );
    println!("last 7d:     ${spent_7d:.4}");
    if let Some(b) = budget {
        println!("today:       {}", b.message);
    }
    if !rollup.daily.is_empty() {
        println!();
        println!("-- daily --");
        for d in &rollup.daily {
            println!(
                "  {}  ${:.4}  {} calls",
                d.period, d.cost_usd, d.llm_calls
            );
        }
    }
    if !rollup.weekly.is_empty() {
        println!();
        println!("-- weekly --");
        for w in &rollup.weekly {
            println!(
                "  {}  ${:.4}  {} calls",
                w.period, w.cost_usd, w.llm_calls
            );
        }
    }
    if !rollup.by_session.is_empty() {
        println!();
        println!("-- by session --");
        for (sid, v) in rollup.by_session.iter().take(10) {
            println!(
                "  {sid}  ${:.4}  {} calls  tokens={}+{}",
                v.cost_usd,
                v.llm_calls,
                v.tokens_prompt,
                v.tokens_completion
            );
        }
    }
    if !rollup.by_agent.is_empty() {
        println!();
        println!("-- by agent --");
        for (aid, v) in rollup.by_agent.iter().take(10) {
            println!(
                "  {aid}  ${:.4}  {} calls",
                v.cost_usd, v.llm_calls
            );
        }
    }
    let open: Vec<_> = circuits.iter().filter(|c| c.open).collect();
    if !open.is_empty() {
        println!();
        println!("-- circuit breakers (open) --");
        for c in open {
            println!("  {} failures={}", c.provider_id, c.failures);
        }
    }
}

