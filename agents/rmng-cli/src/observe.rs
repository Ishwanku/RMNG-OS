use rmng_core::{AuditEntry, AuditLog, AuditTrack, IntegrationRegistry, PermissionGate};

fn audit_track_label(t: AuditTrack) -> String {
    match t {
        AuditTrack::Native => "native".into(),
        AuditTrack::Mcp => "mcp".into(),
        AuditTrack::Plan => "plan".into(),
    }
}
use rmng_core::SessionStore;
use rmng_nervous::{health_check_detailed, load_skill_index, AgentRegistry, NervousConnector};
use std::io::{BufRead, BufReader};

const AUDIT_TAIL: usize = 8;

pub async fn print_observe() {
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
            println!(
                "  {name} [{state}] {} — tools: {}",
                cfg.command,
                cfg.allowed_tools.join(", ")
            );
        }
    }
    println!("  note: MCP children are spawned per request, not persistent");
    println!();

    let audit_path = AuditLog::default_path();
    println!("-- recent audit ({}) --", audit_path.display());
    match tail_audit(&audit_path, AUDIT_TAIL) {
        Ok(entries) if entries.is_empty() => println!("  (no entries)"),
        Ok(entries) => {
            for e in entries {
                let track = e
                    .track
                    .map(audit_track_label)
                    .unwrap_or_else(|| "-".into());
                let dur = e
                    .duration_ms
                    .map(|d| format!("{d}ms"))
                    .unwrap_or_else(|| "-".into());
                println!(
                    "  [{}] {} {} {track} {dur} — {}",
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

fn tail_audit(path: &std::path::Path, n: usize) -> std::io::Result<Vec<AuditEntry>> {
    let file = std::fs::File::open(path)?;
    let lines: Vec<String> = BufReader::new(file).lines().collect::<Result<_, _>>()?;
    let tail: Vec<AuditEntry> = lines
        .iter()
        .rev()
        .take(n)
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();
    Ok(tail.into_iter().rev().collect())
}