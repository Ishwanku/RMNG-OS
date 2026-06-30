use rmng_core::Runtime;
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::UnixListener;
use tracing::info;

fn socket_path() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".rmng/rmngd.sock");
    }
    PathBuf::from("/tmp/rmngd.sock")
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let path = socket_path();
    if path.exists() {
        let _ = std::fs::remove_file(&path);
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create socket dir");
    }

    let listener = UnixListener::bind(&path).expect("bind unix socket");
    info!(socket = %path.display(), "rmngd listening");

    let runtime = Runtime::default();

    loop {
        let (stream, _) = listener.accept().await.expect("accept");
        let runtime = runtime.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stream).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if line.trim().is_empty() {
                    continue;
                }
                match rmng_core::Intent::parse(&line) {
                    Ok(intent) => {
                        let _ = runtime.handle(&intent).await;
                    }
                    Err(e) => tracing::warn!(error = %e, "invalid intent line"),
                }
            }
        });
    }
}
