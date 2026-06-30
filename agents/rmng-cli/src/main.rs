use clap::{Parser, Subcommand};
use rmng_core::{
    daemon_running, send_intent_json, HandleResponse, Intent, PermissionGate, PermissionVerdict,
    Runtime, socket_path,
};
use rmng_nervous::OllamaAdapter;

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
    /// Ask Ollama to produce an intent, then execute via daemon if running
    Ask {
        prompt: String,
        #[arg(long, default_value = "http://127.0.0.1:11434")]
        ollama: String,
        #[arg(long, default_value = "llama3.2")]
        model: String,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        local: bool,
    },
    /// List allowed tools
    Tools,
    /// Show runtime status
    Status,
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
                    "rmngd not running — start: rmngd &\n(socket: {})",
                    socket_path().display()
                );
                1
            } else {
                let json = std::fs::read_to_string(&file).expect("read intent file");
                let intent = Intent::parse(&json).expect("valid intent");
                let compact = serde_json::to_string(&intent).expect("serialize intent");
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
            ollama,
            model,
            dry_run,
            local,
        } => {
            let adapter = OllamaAdapter::new(ollama, model);
            if !adapter.health().await.unwrap_or(false) {
                eprintln!("Ollama not reachable — start with: ollama serve");
                1
            } else {
                let intent = adapter.reason(&prompt).await.expect("ollama intent");
                println!("Intent: {}", serde_json::to_string_pretty(&intent).unwrap());
                if dry_run {
                    0
                } else {
                    execute_intent(&intent, !local).await
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
            println!("rmng 0.1.0 — Phase 5");
            println!("runtime: rmng-core");
            println!("nervous: rmng-nervous (ollama)");
            println!("audit log: {}", rmng_core::AuditLog::default_path().display());
            println!(
                "rmngd: {} ({})",
                if daemon_running() {
                    "running"
                } else {
                    "stopped"
                },
                socket_path().display()
            );
            let adapter = OllamaAdapter::default();
            let ollama = adapter.health().await.unwrap_or(false);
            println!("ollama: {}", if ollama { "reachable" } else { "not running" });
            0
        }
    };
    if code != 0 {
        std::process::exit(code);
    }
}
