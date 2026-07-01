mod observe;

use clap::{Parser, Subcommand};
use rmng_core::{
    daemon_running, parse_incoming, send_intent_json, CoreIntent, HandleResponse, IncomingIntent,
    Intent, PermissionGate, PermissionVerdict, RmngConfig, Runtime, SessionStore, socket_path,
    IntentValidator, IntegrationRegistry,
};
use rmng_nervous::{load_skill, load_skill_index, AgentRouter, NervousConnector, RouteOutcome};

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
    },
    /// Multi-agent session management
    Session {
        #[command(subcommand)]
        action: SessionCommands,
    },
    /// List allowed tools
    Tools,
    /// Show runtime status
    Status,
    /// Runtime observability — integrations, agents, audit tail, MCP allowlist
    Observe,
}

#[derive(Subcommand)]
enum SessionCommands {
    /// Create a new session
    New,
    /// List session ids
    List,
    /// Show session details
    Show { id: String },
}

async fn dispatch_core_intent(intent: &CoreIntent, dry_run: bool) -> i32 {
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
                IncomingIntent::Core(intent) => dispatch_core_intent(&intent, false).await,
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
        } => {
            if agent.is_some() && skill.is_some() {
                eprintln!("use either --agent or --skill, not both");
                std::process::exit(1);
            }

            if let Some(agent_id) = agent {
                let router = AgentRouter::load();
                match router
                    .ask_routed(session.as_deref(), &agent_id, &prompt)
                    .await
                {
                    Ok(outcome) => {
                        if outcome.is_handoff() {
                            if let RouteOutcome::Handoff {
                                from_agent,
                                to_agent,
                                from_layer,
                                to_layer,
                                reason,
                                ..
                            } = &outcome
                            {
                                println!(
                                    "handoff: {from_agent} ({from_layer}) → {to_agent} ({to_layer}) — {reason}"
                                );
                            }
                        }
                        dispatch_core_intent(&outcome.intent(), dry_run).await
                    }
                    Err(e) => {
                        eprintln!("agent router: {e}");
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

                let connector = NervousConnector::load();
                let skill_ref = loaded_skill.as_ref();
                let skill_name = skill.as_deref();
                match connector.reason_core(&prompt, skill_name, skill_ref).await {
                    Ok(intent) => dispatch_core_intent(&intent, dry_run).await,
                    Err(e) => {
                        eprintln!("nervous system: {e}");
                        1
                    }
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
            SessionCommands::List => {
                let store = SessionStore::default_store();
                match store.list_ids() {
                    Ok(ids) => {
                        if ids.is_empty() {
                            println!("(no sessions)");
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
            println!("rmng 0.1.0 — Sprint 3 (multi-level agents + sessions)");
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
        Commands::Observe => {
            observe::print_observe();
            0
        }
    };
    if code != 0 {
        std::process::exit(code);
    }
}
