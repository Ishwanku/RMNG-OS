use rmng_core::{parse_incoming, Runtime};
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;
use tracing::info;

fn socket_path() -> PathBuf {
    rmng_core::socket_path()
}

async fn handle_connection(stream: tokio::net::UnixStream, runtime: Runtime) {
    let (reader, mut writer) = stream.into_split();
    let mut lines = BufReader::new(reader).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        if line.trim().is_empty() {
            continue;
        }
        let response = match parse_incoming(&line) {
            Ok(incoming) => match runtime.handle_incoming(&incoming).await {
                Ok(resp) => serde_json::to_string(&resp).unwrap_or_else(|e| {
                    serde_json::to_string(&rmng_core::HandleResponse::failure(e.to_string()))
                        .unwrap()
                }),
                Err(e) => serde_json::to_string(&rmng_core::HandleResponse::failure(e.to_string()))
                    .unwrap(),
            },
            Err(e) => serde_json::to_string(&rmng_core::HandleResponse::failure(e.to_string()))
                .unwrap(),
        };
        if writer
            .write_all(format!("{response}\n").as_bytes())
            .await
            .is_err()
        {
            break;
        }
    }
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

    let runtime = Runtime::bootstrap().unwrap_or_else(|e| {
        tracing::warn!(error = %e, "bootstrap failed — starting in degraded mode");
        Runtime::default()
    });

    loop {
        let (stream, _) = listener.accept().await.expect("accept");
        let runtime = runtime.clone();
        tokio::spawn(handle_connection(stream, runtime));
    }
}