mod audit_cmd;
mod llm_cmd;
mod observe;

use clap::{Parser, Subcommand};
use rmng_core::{
    daemon_running, parse_incoming, parse_provider_str, persist_dispatch_to_session,
    send_intent_json, CoreIntent, HandleResponse, IncomingIntent, Intent, PermissionGate,
    PermissionVerdict, RmngConfig, Runtime, SessionStore, socket_path, IntentValidator,
    IntegrationRegistry,
};
use rmng_core::OrchestrationSnapshot;
use rmng_nervous::{
    list_supported_providers,
    load_skill, load_skill_index, run_provider_matrix, AgentRouter,
    NervousConnector, RouteOutcome,
};

#[derive(Parser)]
#[command(name = "rmng", about = "RMNG-OS CLI", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Parse and validate a JSON intent file (v1 or v2 CoreIntent)
    Intent {
        #[arg(short, long)]
        file: String,
    },
    /// Execute locally (no daemon)
    Run {
        #[arg(short, long)]
        file: String,
    },
    /// Send intent to rmngd and print response
    Send {
        #[arg(short, long)]
        file: String,
    },
    /// Nervous system: config-driven LLM → v2 CoreIntent → rmngd dispatch
    Ask {
        prompt: String,
        #[arg(short = 's', long = "skill", help = "Load skills/<name>/SKILL.md as context")]
        skill: Option<String>,
        #[arg(short = 'a', long = "agent", help = "Route via agents/definitions/<name>.yaml")]
        agent: Option<String>,
        #[arg(long, help = "Session id for multi-agent handoff persistence")]
        session: Option<String>,
        #[arg(long, help = "Produce intent only; do not dispatch to rmngd")]
        dry_run: bool,
        #[arg(long, help = "Override LLM model id (from catalog or provider docs)")]
        model: Option<String>,
        #[arg(long, help = "Override LLM provider (grok, google, anthropic, ollama, …)")]
        provider: Option<String>,
        #[arg(long, help = "Use named profile from ~/.rmng/config.toml")]
        profile: Option<String>,
        #[arg(
            long,
            help = "Auto-continue: dispatch executable intents and re-ask until plan.only or max steps (requires --session)"
        )]
        auto_continue: bool,
        #[arg(long, default_value = "3", help = "Max auto-continue steps (with --auto-continue)")]
        max_steps: u32,
    },
    /// Multi-agent session management
    Session {
        #[command(subcommand)]
        action: SessionCommands,
    },
    /// Explicit agent handoff within a session
    Handoff {
        #[arg(long, help = "Session id (required)")]
        session: String,
        #[arg(long = "from", help = "Source agent id (ignored when --chain is set)")]
        from_agent: Option<String>,
        #[arg(long = "to", help = "Target agent id (ignored when --chain is set)")]
        to_agent: Option<String>,
        #[arg(
            long,
            help = "Comma-separated handoff chain, e.g. swarm-coordinator,repo-keeper,runtime-executor"
        )]
        chain: Option<String>,
        #[arg(long, default_value = "explicit handoff")]
        reason: String,
        #[arg(help = "Task prompt for the target agent")]
        prompt: String,
        #[arg(long, help = "Produce intent only; do not dispatch")]
        dry_run: bool,
    },
    /// List allowed tools
    Tools,
    /// Show runtime status
    Status,
    /// Runtime observability — LLM health, budgets, circuits, MCP resources, audit tail
    Observe {
        #[arg(long, help = "Cost + MCP resource rollups (LLM spend, peak RSS, circuits)")]
        cost: bool,
        #[arg(long, help = "Structured JSON (schema v1): cost_rollup, resource_rollup, budgets, circuits")]
        json: bool,
    },
    /// Tamper-evident audit log tools
    Audit {
        #[command(subcommand)]
        action: AuditCommands,
    },
    /// LLM provider management
    Llm {
        #[command(subcommand)]
        action: LlmCommands,
    },
}

#[derive(Subcommand)]
enum LlmCommands {
    /// Check health of the configured LLM provider
    Health {
        #[arg(long, help = "JSON output for monitoring")]
        json: bool,
    },
    /// Run provider validation matrix (uses env API keys)
    Matrix,
    /// List all supported LLM providers (legacy wired list)
    List,
    /// Show active config, catalog path, and profiles
    Show,
    /// List providers from editable llm-catalog.toml
    Providers,
    /// List catalog models for a provider (default: active provider)
    Models {
        #[arg(long, help = "Provider id: google, grok, anthropic, ollama, …")]
        provider: Option<String>,
        #[arg(long, help = "Include image/audio/embedding models")]
        specialized: bool,
        #[arg(long, help = "Query provider /models API and compare with catalog")]
        live: bool,
        #[arg(long, help = "Show input/output $/1M token pricing")]
        pricing: bool,
    },
    /// Switch active [[llm.profiles]] preset in config
    Use { name: String },
    /// Copy repo catalog to ~/.rmng/llm-catalog.toml
    Setup,
    /// Compare live provider APIs against local catalog (drift report)
    SyncCatalog {
        #[arg(long, help = "Include specialized models in comparison")]
        specialized: bool,
        #[arg(long, help = "Merge live-only models into ~/.rmng/llm-catalog.toml")]
        apply: bool,
    },
}

#[derive(Subcommand)]
enum AuditCommands {
    /// Verify audit hash chain (exit 1 if tampered); --stats adds cost + resource rollups
    Verify {
        #[arg(long, help = "JSON output for CI/cron")]
        json: bool,
        #[arg(long, help = "Category stats, LLM cost + MCP resource rollups")]
        stats: bool,
    },
}

#[derive(Subcommand)]
enum SessionCommands {
    /// Create a new session
    New,
    /// List session ids (use --verbose for active/stale status)
    List {
        #[arg(short, long)]
        verbose: bool,
    },
    /// Remove sessions older than N days (ADR-018)
    Prune {
        #[arg(long, default_value = "30")]
        older_than_days: u32,
        #[arg(long)]
        dry_run: bool,
    },
    /// Show session details
    Show { id: String },
    /// Set a shared context key (JSON value)
    SetContext {
        id: String,
        key: String,
        /// JSON value (e.g. "\"hello\"" or "{\"repo\":\"RMNG-OS\"}")
        value: String,
    },
}

fn nervous_connector_for_ask(
    provider: Option<&str>,
    model: Option<&str>,
    profile: Option<&str>,
) -> NervousConnector {
    let base = RmngConfig::load();
    let prov = provider.and_then(|s| parse_provider_str(s).ok());
    let cfg = base.with_llm_overrides(
        prov,
        model.map(str::to_string),
        profile.map(str::to_string),
    );
    NervousConnector::from_config(cfg)
}

fn maybe_persist_session_result(
    session_id: Option<&str>,
    intent: &CoreIntent,
    resp: &HandleResponse,
) {
    let sid = session_id
        .or_else(|| intent.metadata().and_then(|m| m.session_id.as_deref()));
    let Some(sid) = sid else {
        return;
    };
    let store = SessionStore::default_store();
    if let Err(e) = persist_dispatch_to_session(&store, sid, intent, resp) {
        eprintln!("session write-back: {e}");
    }
}


fn print_handoff_outcome(outcome: &RouteOutcome) {
    if let Some(summary) = outcome.chain_outcome_summary() {
        println!("{summary}");
    }
}

fn print_orchestration_failure_context(session_id: &str) {
    let store = SessionStore::default_store();
    let Ok(session) = store.load(session_id) else {
        return;
    };
    let Some(orch) = session.shared_context.get("orchestration") else {
        return;
    };
    let snap = OrchestrationSnapshot::from_value(orch);
    if snap.has_failures() || snap.status.as_deref() == Some("completed_with_skips") {
        eprintln!("--- chain recovery context ---");
        eprintln!("{}", snap.cli_failure_report());
        let hints = snap.recovery_hints();
        if !hints.is_empty() {
            eprintln!("{hints}");
        }
        eprintln!("--- end chain context ---");
    }
}

async fn ask_with_auto_continue(
    router: &AgentRouter,
    session_id: &str,
    start_agent: &str,
    prompt: &str,
    max_steps: u32,
    dry_run: bool,
) -> i32 {
    let mut agent_id = start_agent.to_string();
    let mut step_prompt = prompt.to_string();
    let mut last_code = 0;

    for step in 0..max_steps {
        let outcome = match router
            .ask_routed(Some(session_id), &agent_id, &step_prompt)
            .await
        {
            Ok(o) => o,
            Err(e) => {
                eprintln!("agent router (step {}): {e}", step + 1);
                print_orchestration_failure_context(session_id);
                return 1;
            }
        };

        if outcome.is_handoff() {
            print_handoff_outcome(&outcome);
        }

        let mut intent = outcome.intent();
        let handoff_from = outcome.handoff_from_agent();
        AgentRouter::enrich_intent_metadata(&mut intent, Some(session_id), handoff_from);

        let executable = intent.is_executable();
        last_code = dispatch_core_intent(&intent, dry_run, Some(session_id)).await;

        if !executable {
            return last_code;
        }
        if last_code != 0 {
            eprintln!("auto-continue: dispatch failed at step {}, stopping", step + 1);
            return last_code;
        }

        if let Some(next) = outcome.final_agent() {
            agent_id = next.to_string();
        }
        step_prompt = "Continue the orchestration. If specialist work is complete, emit plan.only with handoff_return_to swarm-coordinator summarizing recent_tool_results. Otherwise execute the next required tool. Do not repeat successful tools.".to_string();

        if step + 1 >= max_steps {
            println!("auto-continue: reached max steps ({max_steps})");
            return last_code;
        }
    }
    last_code
}

async fn dispatch_core_intent(
    intent: &CoreIntent,
    dry_run: bool,
    session_id: Option<&str>,
) -> i32 {
    println!(
        "Intent: {}",
        serde_json::to_string_pretty(intent).expect("serialize intent")
    );

    if dry_run {
        return 0;
    }

    match intent {
        CoreIntent::PlanOnly { reasoning, .. } => {
            println!("{reasoning}");
            0
        }
        CoreIntent::ToolExecute { .. } | CoreIntent::McpProxy { .. } => {
            if !daemon_running() {
                eprintln!(
                    "rmngd not running — start: systemctl --user start rmngd\n(socket: {})",
                    socket_path().display()
                );
                return 1;
            }
            let json = serde_json::to_string(intent).expect("serialize core intent");
            match send_intent_json(&json).await {
                Ok(line) => {
                    let resp: HandleResponse = serde_json::from_str(line.trim())
                        .unwrap_or_else(|e| HandleResponse::failure(e.to_string()));
                    maybe_persist_session_result(session_id, intent, &resp);
                    print_response(&resp)
                }
                Err(e) => {
                    eprintln!("{e}");
                    1
                }
            }
        }
    }
}

fn print_response(resp: &HandleResponse) -> i32 {
    if let Some(result) = &resp.tool_result {
        print!("{}", result.output);
        if !resp.ok || !result.success {
            return result.exit_code.unwrap_or(1);
        }
        return 0;
    }
    if resp.ok {
        if let Some(kind) = &resp.kind {
            println!("OK: {kind:?}");
        }
        return 0;
    }
    eprintln!("{}", resp.error.as_deref().unwrap_or("unknown error"));
    1
}

async fn execute_intent(intent: &Intent, prefer_daemon: bool) -> i32 {
    if prefer_daemon && daemon_running() {
        let json = serde_json::to_string(intent).expect("serialize intent");
        match send_intent_json(&json).await {
            Ok(line) => {
                let resp: HandleResponse = serde_json::from_str(line.trim())
                    .unwrap_or_else(|e| HandleResponse::failure(e.to_string()));
                return print_response(&resp);
            }
            Err(e) => {
                eprintln!("{e}");
                return 1;
            }
        }
    }
    let runtime = Runtime::default();
    match runtime.handle_response(intent).await {
        Ok(resp) => print_response(&resp),
        Err(e) => {
            eprintln!("{e}");
            1
        }
    }
}

fn evaluate_intent_file(json: &str) -> i32 {
    let gate = PermissionGate::default();
    match parse_incoming(json) {
        Ok(IncomingIntent::Core(intent)) => {
            let validator = match IntegrationRegistry::load() {
                Ok(reg) => IntentValidator::new(reg).ok(),
                Err(_) => None,
            };
            if let Some(v) = &validator {
                if let Err(e) = v.validate(&intent) {
                    eprintln!("INVALID: {e}");
                    return 1;
                }
            }
            match gate.evaluate_core(&intent) {
                PermissionVerdict::Allow => {
                    let action = match &intent {
                        CoreIntent::ToolExecute { target, .. } => format!("tool.execute:{target}"),
                        CoreIntent::McpProxy { mcp_server, mcp_tool, .. } => {
                            format!("mcp.proxy:{mcp_server}.{mcp_tool}")
                        }
                        CoreIntent::PlanOnly { .. } => "plan.only".into(),
                    };
                    println!("OK: {action} (v2 CoreIntent)");
                    0
                }
                PermissionVerdict::Deny(reason) => {
                    eprintln!("DENIED: {reason}");
                    1
                }
            }
        }
        Ok(IncomingIntent::V1(intent)) => match gate.evaluate(&intent) {
            PermissionVerdict::Allow => {
                println!("OK: {:?} (v1)", intent.kind);
                0
            }
            PermissionVerdict::Deny(reason) => {
                eprintln!("DENIED: {reason}");
                1
            }
        },
        Err(e) => {
            eprintln!("INVALID: {e}");
            1
        }
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    let code = match cli.command {
        Commands::Intent { file } => {
            let json = std::fs::read_to_string(&file).expect("read intent file");
            evaluate_intent_file(&json)
        }
        Commands::Run { file } => {
            let json = std::fs::read_to_string(&file).expect("read intent file");
            let incoming = parse_incoming(&json).expect("valid intent");
            match incoming {
                IncomingIntent::V1(intent) => execute_intent(&intent, false).await,
                IncomingIntent::Core(intent) => dispatch_core_intent(&intent, false, None).await,
            }
        }
        Commands::Send { file } => {
            if !daemon_running() {
                eprintln!(
                    "rmngd not running — start: systemctl --user start rmngd\n(socket: {})",
                    socket_path().display()
                );
                1
            } else {
                let json = std::fs::read_to_string(&file).expect("read intent file");
                let incoming = parse_incoming(&json).expect("valid intent");
                let compact = match &incoming {
                    IncomingIntent::V1(intent) => {
                        serde_json::to_string(intent).expect("serialize intent")
                    }
                    IncomingIntent::Core(intent) => {
                        serde_json::to_string(intent).expect("serialize core intent")
                    }
                };
                match send_intent_json(&compact).await {
                    Ok(line) => {
                        let resp: HandleResponse = serde_json::from_str(line.trim())
                            .unwrap_or_else(|e| HandleResponse::failure(e.to_string()));
                        print_response(&resp)
                    }
                    Err(e) => {
                        eprintln!("{e}");
                        1
                    }
                }
            }
        }
        Commands::Ask {
            prompt,
            skill,
            agent,
            session,
            dry_run,
            model,
            provider,
            profile,
            auto_continue,
            max_steps,
        } => {
            if agent.is_some() && skill.is_some() {
                eprintln!("use either --agent or --skill, not both");
                std::process::exit(1);
            }

            if let Some(agent_id) = agent {
                if auto_continue && session.is_none() {
                    eprintln!("--auto-continue requires --session");
                    std::process::exit(1);
                }
                let connector = nervous_connector_for_ask(
                    provider.as_deref(),
                    model.as_deref(),
                    profile.as_deref(),
                );
                let registry = rmng_nervous::AgentRegistry::load().unwrap_or_else(|e| {
                    tracing::warn!(error = %e, "agent registry load failed");
                    rmng_nervous::AgentRegistry::load_from(std::path::Path::new("/nonexistent"))
                        .unwrap()
                });
                let router = AgentRouter::with_registry(registry, connector);
                if auto_continue {
                    if let Some(ref sid) = session {
                        let code = ask_with_auto_continue(
                            &router,
                            sid,
                            &agent_id,
                            &prompt,
                            max_steps,
                            dry_run,
                        )
                        .await;
                        std::process::exit(code);
                    }
                }
                match router
                    .ask_routed(session.as_deref(), &agent_id, &prompt)
                    .await
                {
                    Ok(outcome) => {
                        if outcome.is_handoff() {
                            match &outcome {
                                RouteOutcome::HandoffChain { chain, hops, reason, .. } => {
                                    println!("handoff-chain ({reason}): {}", chain.join(" → "));
                                    for hop in hops {
                                        println!(
                                            "  hop: {} → {} — {}",
                                            hop.from_agent, hop.to_agent, hop.reason
                                        );
                                    }
                                }
                                RouteOutcome::Handoff {
                                    from_agent,
                                    to_agent,
                                    from_layer,
                                    to_layer,
                                    reason,
                                    ..
                                } => {
                                    println!(
                                        "handoff: {from_agent} ({from_layer}) → {to_agent} ({to_layer}) — {reason}"
                                    );
                                }
                                _ => {}
                            }
                        }
                        let mut intent = outcome.intent();
                        let handoff_from = outcome.handoff_from_agent();
                        if let Some(ref sid) = session {
                            AgentRouter::enrich_intent_metadata(
                                &mut intent,
                                Some(sid.as_str()),
                                handoff_from,
                            );
                        }
                        dispatch_core_intent(&intent, dry_run, session.as_deref()).await
                    }
                    Err(e) => {
                        eprintln!("agent router: {e}");
                        if let Some(ref sid) = session {
                            print_orchestration_failure_context(sid);
                        }
                        1
                    }
                }
            } else {
                let loaded_skill = match skill.as_deref() {
                    Some(name) => match load_skill(name) {
                        Ok(s) => Some(s),
                        Err(e) => {
                            eprintln!("skill error: {e}");
                            std::process::exit(1);
                        }
                    },
                    None => None,
                };

                let connector = nervous_connector_for_ask(
                    provider.as_deref(),
                    model.as_deref(),
                    profile.as_deref(),
                );
                let skill_ref = loaded_skill.as_ref();
                let skill_name = skill.as_deref();
                match connector.reason_core(&prompt, skill_name, skill_ref).await {
                    Ok(intent) => dispatch_core_intent(&intent, dry_run, None).await,
                    Err(e) => {
                        eprintln!("nervous system: {e}");
                        1
                    }
                }
            }
        }

        Commands::Handoff {
            session,
            from_agent,
            to_agent,
            chain,
            reason,
            prompt,
            dry_run,
        } => {
            let router = AgentRouter::load();
            let chain_agents: Option<Vec<String>> = chain.as_ref().map(|c| {
                c.split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
                    .collect()
            });
            let handoff_result = if let Some(agents) = chain_agents.as_ref() {
                if agents.len() < 2 {
                    eprintln!("--chain requires at least two agents");
                    std::process::exit(1);
                }
                router
                    .handoff_chain(&session, agents, &prompt, &reason)
                    .await
            } else {
                let from = from_agent.as_deref().unwrap_or_else(|| {
                    eprintln!("--from is required unless --chain is set");
                    std::process::exit(1);
                });
                let to = to_agent.as_deref().unwrap_or_else(|| {
                    eprintln!("--to is required unless --chain is set");
                    std::process::exit(1);
                });
                router.handoff(&session, from, to, &prompt, &reason).await
            };
            match handoff_result {
                Ok(outcome) => {
                    match &outcome {
                        RouteOutcome::HandoffChain { chain, hops, reason, .. } => {
                            println!("handoff-chain ({reason}): {}", chain.join(" → "));
                            for hop in hops {
                                println!(
                                    "  hop: {} → {} — {}",
                                    hop.from_agent, hop.to_agent, hop.reason
                                );
                            }
                        }
                        RouteOutcome::Handoff {
                            from_agent,
                            to_agent,
                            from_layer,
                            to_layer,
                            reason,
                            ..
                        } => {
                            println!(
                                "handoff: {from_agent} ({from_layer}) → {to_agent} ({to_layer}) — {reason}"
                            );
                        }
                        _ => {}
                    }
                    let mut intent = outcome.intent();
                    let handoff_from = outcome.handoff_from_agent();
                    AgentRouter::enrich_intent_metadata(
                        &mut intent,
                        Some(session.as_str()),
                        handoff_from,
                    );
                    dispatch_core_intent(&intent, dry_run, Some(session.as_str())).await
                }
                Err(e) => {
                    eprintln!("handoff: {e}");
                    print_orchestration_failure_context(session.as_str());
                    1
                }
            }
        }
        Commands::Session { action } => match action {
            SessionCommands::New => {
                let store = SessionStore::default_store();
                match store.create() {
                    Ok(session) => {
                        println!("session: {}", session.id);
                        println!("path: {}/{}.json", store.root().display(), session.id);
                        0
                    }
                    Err(e) => {
                        eprintln!("{e}");
                        1
                    }
                }
            }
            SessionCommands::List { verbose } => {
                let store = SessionStore::default_store();
                match store.list_ids() {
                    Ok(ids) => {
                        if ids.is_empty() {
                            println!("(no sessions)");
                        } else if verbose {
                            for id in ids {
                                match store.load(&id) {
                                    Ok(session) => {
                                        let status = session.lifecycle_label();
                                        println!(
                                            "{id}  {status}  updated={}  handoffs={}",
                                            session.updated_at.to_rfc3339(),
                                            session.handoff_history.len()
                                        );
                                    }
                                    Err(e) => println!("{id}  error: {e}"),
                                }
                            }
                        } else {
                            for id in ids {
                                println!("{id}");
                            }
                        }
                        0
                    }
                    Err(e) => {
                        eprintln!("{e}");
                        1
                    }
                }
            }
            SessionCommands::Prune {
                older_than_days,
                dry_run,
            } => {
                let store = SessionStore::default_store();
                match store.prune_older_than(older_than_days, dry_run) {
                    Ok(removed) => {
                        if removed.is_empty() {
                            println!("(no sessions older than {older_than_days} days)");
                        } else if dry_run {
                            println!(
                                "would prune {} session(s) older than {older_than_days} days:",
                                removed.len()
                            );
                            for id in removed {
                                println!("  {id}");
                            }
                        } else {
                            println!(
                                "pruned {} session(s) older than {older_than_days} days",
                                removed.len()
                            );
                            for id in removed {
                                println!("  {id}");
                            }
                        }
                        0
                    }
                    Err(e) => {
                        eprintln!("{e}");
                        1
                    }
                }
            }

            SessionCommands::SetContext { id, key, value } => {
                let store = SessionStore::default_store();
                match store.load(&id) {
                    Ok(mut session) => {
                        let parsed: serde_json::Value = match serde_json::from_str(&value) {
                            Ok(v) => v,
                            Err(_) => serde_json::Value::String(value.clone()),
                        };
                        match store.set_context(&mut session, &key, parsed) {
                            Ok(()) => {
                                println!("OK: context[{key}] set for session {id}");
                                0
                            }
                            Err(e) => {
                                eprintln!("{e}");
                                1
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("{e}");
                        1
                    }
                }
            }
            SessionCommands::Show { id } => {
                let store = SessionStore::default_store();
                match store.load(&id) {
                    Ok(session) => {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&session).expect("serialize session")
                        );
                        0
                    }
                    Err(e) => {
                        eprintln!("{e}");
                        1
                    }
                }
            }
        },
        Commands::Tools => {
            match IntegrationRegistry::load() {
                Ok(reg) => {
                    for t in reg.allowed_tool_names() {
                        let handler = if rmng_core::tools::registered_tools().contains(&t.as_str()) {
                            "handler"
                        } else {
                            "manifest-only"
                        };
                        println!("{t} ({handler})");
                    }
                }
                Err(e) => {
                    eprintln!("registry: {e}");
                    for t in rmng_core::tools::registered_tools() {
                        println!("{t}");
                    }
                }
            }
            0
        }
        Commands::Status => {
            let cfg = RmngConfig::load();
            let connector = NervousConnector::from_config(cfg);
            println!("rmng 0.1.0 — Sprints 19–22 (ops, resources, security hardening)");
            if let Ok(reg) = IntegrationRegistry::load() {
                println!(
                    "integrations: {} manifests, {} tools",
                    reg.manifests().len(),
                    reg.allowed_tool_names().len()
                );
            }
            if let Ok(index) = load_skill_index() {
                println!("skills index: {} (progressive disclosure)", index.len());
            }
            if let Ok(agents) = rmng_nervous::AgentRegistry::load() {
                println!("agents: {} (L1–L4)", agents.agent_ids().len());
            }
            let store = SessionStore::default_store();
            if let Ok(ids) = store.list_ids() {
                println!("sessions: {}", ids.len());
            }
            println!("runtime: rmng-core");
            println!("nervous: {} ({})", connector.provider_label(), RmngConfig::config_path().display());
            println!("audit log: {}", rmng_core::AuditLog::default_path().display());
            println!(
                "rmngd: {} ({})",
                if daemon_running() { "running" } else { "stopped" },
                socket_path().display()
            );
            0
        }
        Commands::Observe { cost, json } => {
            observe::print_observe(cost, json).await;
            0
        }
        Commands::Audit { action } => match action {
            AuditCommands::Verify { json, stats } => audit_cmd::run_verify(json, stats),
        },
        Commands::Llm { action } => match action {
            LlmCommands::Health { json } => llm_cmd::run_health(json).await,
            LlmCommands::Matrix => {
                println!("Provider matrix (env keys only — never commit keys to config):");
                println!();
                let rows = run_provider_matrix().await;
                let mut failures = 0u32;
                for row in &rows {
                    let health = row
                        .health_ok
                        .map(|h| if h { "ok" } else { "FAIL" })
                        .unwrap_or("skip");
                    let json = row
                        .json_ok
                        .map(|j| if j { "ok" } else { "FAIL" })
                        .unwrap_or("skip");
                    let env = row.env_var.as_deref().unwrap_or("-");
                    println!(
                        "{:<8} key={:<5} health={:<4} json={:<4} env={env} — {}",
                        row.provider, row.key_set, health, json, row.detail
                    );
                    if row.key_set && (row.health_ok == Some(false) || row.json_ok == Some(false)) {
                        failures += 1;
                    }
                }
                if failures > 0 { 1 } else { 0 }
            }
            LlmCommands::List => {
                println!("Supported LLM providers:");
                for (id, desc, wired) in list_supported_providers() {
                    let tag = if wired { "wired" } else { "planned" };
                    println!("  {id:<12} [{tag}] {desc}");
                }
                0
            }
            LlmCommands::Show => {
                llm_cmd::print_show();
                0
            }
            LlmCommands::Providers => {
                llm_cmd::print_providers();
                0
            }
            LlmCommands::Models {
                provider,
                specialized,
                live,
                pricing,
            } => {
                llm_cmd::print_models(provider.as_deref(), specialized, live, pricing);
                0
            }
            LlmCommands::Use { name } => llm_cmd::run_use(&name),
            LlmCommands::Setup => llm_cmd::run_setup(),
            LlmCommands::SyncCatalog { specialized, apply } => {
                llm_cmd::run_sync_catalog(specialized, apply)
            }
        }
    };
    if code != 0 {
        std::process::exit(code);
    }
}
