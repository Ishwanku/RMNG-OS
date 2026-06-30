use clap::{Parser, Subcommand};
use rmng_core::{
    daemon_running, parse_incoming, send_intent_json, CoreIntent, HandleResponse, Intent,
    PermissionGate, PermissionVerdict, RmngConfig, Runtime, socket_path,
};
use rmng_nervous::{load_skill, NervousConnector};

#[derive(Parser)]
#[command(name = "rmng", about = "RMNG-OS CLI", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Parse and validate a JSON intent file (permission check only)
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
        #[arg(long, help = "Produce intent only; do not dispatch to rmngd")]
        dry_run: bool,
    },
    /// List allowed tools
    Tools,
    /// Show runtime status
    Status,
}

/// Dispatch v2 intent to rmngd only — CLI never executes tools locally.
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

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    let code = match cli.command {
        Commands::Intent { file } => {
            let json = std::fs::read_to_string(&file).expect("read intent file");
            let intent = Intent::parse(&json).expect("valid intent");
            let gate = PermissionGate::default();
            match gate.evaluate(&intent) {
                PermissionVerdict::Allow => {
                    println!("OK: {:?}", intent.kind);
                    0
                }
                PermissionVerdict::Deny(reason) => {
                    eprintln!("DENIED: {reason}");
                    1
                }
            }
        }
        Commands::Run { file } => {
            let json = std::fs::read_to_string(&file).expect("read intent file");
            let intent = Intent::parse(&json).expect("valid intent");
            execute_intent(&intent, false).await
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
                    rmng_core::IncomingIntent::V1(intent) => {
                        serde_json::to_string(intent).expect("serialize intent")
                    }
                    rmng_core::IncomingIntent::Core(intent) => {
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
            dry_run,
        } => {
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
            match connector
                .reason_core(&prompt, skill_name, skill_ref)
                .await
            {
                Ok(intent) => dispatch_core_intent(&intent, dry_run).await,
                Err(e) => {
                    eprintln!("nervous system: {e}");
                    1
                }
            }
        }
        Commands::Tools => {
            for t in rmng_core::tools::list() {
                println!("{t}");
            }
            0
        }
        Commands::Status => {
            let cfg = RmngConfig::load();
            let connector = NervousConnector::from_config(cfg);
            println!("rmng 0.1.0 — Phase 6c (BYO-LLM + skills)");
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
    };
    if code != 0 {
        std::process::exit(code);
    }
}
