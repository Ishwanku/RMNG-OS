use crate::RmngError;
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

pub fn socket_path() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".rmng/rmngd.sock");
    }
    PathBuf::from("/tmp/rmngd.sock")
}

pub fn daemon_running() -> bool {
    socket_path().exists()
}

pub async fn send_intent_json(json: &str) -> Result<String, RmngError> {
    let path = socket_path();
    if !path.exists() {
        return Err(RmngError::Ipc(format!(
            "rmngd not running (no socket at {})",
            path.display()
        )));
    }
    let mut stream = UnixStream::connect(&path)
        .await
        .map_err(|e| RmngError::Ipc(e.to_string()))?;
    let line = if json.ends_with('\n') {
        json.to_string()
    } else {
        format!("{json}\n")
    };
    stream
        .write_all(line.as_bytes())
        .await
        .map_err(|e| RmngError::Ipc(e.to_string()))?;
    stream
        .flush()
        .await
        .map_err(|e| RmngError::Ipc(e.to_string()))?;

    let mut reader = BufReader::new(stream);
    let mut response = String::new();
    reader
        .read_line(&mut response)
        .await
        .map_err(|e| RmngError::Ipc(e.to_string()))?;
    Ok(response)
}
