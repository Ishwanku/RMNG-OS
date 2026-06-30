use clap::{Parser, Subcommand};
use rmng_core::{Intent, PermissionGate, PermissionVerdict};
use tracing::info;

#[derive(Parser)]
#[command(name = "rmng", about = "RMNG-OS CLI", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Parse and validate a JSON intent file
    Intent {
        #[arg(short, long)]
        file: String,
    },
    /// Show runtime status (stub)
    Status,
}

fn main() {
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
                PermissionVerdict::Allow => {
                    info!(intent_id = %intent.intent_id, tool = ?intent.tool, "intent allowed");
                    println!("OK: {:?}", intent.kind);
                }
                PermissionVerdict::Deny(reason) => {
                    eprintln!("DENIED: {reason}");
                    std::process::exit(1);
                }
            }
        }
        Commands::Status => {
            println!("rmng 0.1.0 — Phase 5 scaffold");
            println!("runtime: rmng-core");
            println!("daemon: rmngd (not running)");
        }
    }
}
