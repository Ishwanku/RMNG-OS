use clap::{Parser, Subcommand};
use rmng_core::{Intent, PermissionGate, PermissionVerdict, Runtime};
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
    /// Execute an intent through permission gate + tool dispatch
    Run {
        #[arg(short, long)]
        file: String,
    },
    /// Ask Ollama to produce an intent, then execute it
    Ask {
        prompt: String,
        #[arg(long, default_value = "http://127.0.0.1:11434")]
        ollama: String,
        #[arg(long, default_value = "llama3.2")]
        model: String,
        #[arg(long)]
        dry_run: bool,
    },
    /// List allowed tools
    Tools,
    /// Show runtime status
    Status,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    match cli.command {
        Commands::Intent { file } => {
            let json = std::fs::read_to_string(&file).expect("read intent file");
            let intent = Intent::parse(&json).expect("valid intent");
            let gate = PermissionGate::default();
            match gate.evaluate(&intent) {
                PermissionVerdict::Allow => println!("OK: {:?}", intent.kind),
                PermissionVerdict::Deny(reason) => {
                    eprintln!("DENIED: {reason}");
                    std::process::exit(1);
                }
            }
        }
        Commands::Run { file } => {
            let json = std::fs::read_to_string(&file).expect("read intent file");
            let intent = Intent::parse(&json).expect("valid intent");
            let runtime = Runtime::default();
            match runtime.handle(&intent).await {
                Ok(Some(result)) => {
                    print!("{}", result.output);
                    if !result.success {
                        std::process::exit(result.exit_code.unwrap_or(1) as i32);
                    }
                }
                Ok(None) => println!("OK: {:?}", intent.kind),
                Err(e) => {
                    eprintln!("{e}");
                    std::process::exit(1);
                }
            }
        }
        Commands::Ask {
            prompt,
            ollama,
            model,
            dry_run,
        } => {
            let adapter = OllamaAdapter::new(ollama, model);
            if !adapter.health().await.unwrap_or(false) {
                eprintln!("Ollama not reachable — start with: ollama serve");
                std::process::exit(1);
            }
            let intent = adapter.reason(&prompt).await.expect("ollama intent");
            println!("Intent: {}", serde_json::to_string_pretty(&intent).unwrap());
            if dry_run {
                return;
            }
            let runtime = Runtime::default();
            match runtime.handle(&intent).await {
                Ok(Some(result)) => {
                    print!("{}", result.output);
                    if !result.success {
                        std::process::exit(result.exit_code.unwrap_or(1) as i32);
                    }
                }
                Ok(None) => {}
                Err(e) => {
                    eprintln!("{e}");
                    std::process::exit(1);
                }
            }
        }
        Commands::Tools => {
            for t in rmng_core::tools::list() {
                println!("{t}");
            }
        }
        Commands::Status => {
            println!("rmng 0.1.0 — Phase 5");
            println!("runtime: rmng-core");
            println!("nervous: rmng-nervous (ollama)");
            println!("audit log: {}", rmng_core::AuditLog::default_path().display());
            let adapter = OllamaAdapter::default();
            let ollama = adapter.health().await.unwrap_or(false);
            println!("ollama: {}", if ollama { "reachable" } else { "not running" });
        }
    }
}
