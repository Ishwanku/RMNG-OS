use rmngd::orchestration::DaemonOrchestrator;
use rmng_core::{
    parse_daemon_line, DaemonLine, HandleResponse, IncomingIntent, OrchestrationContinueResponse,
    Runtime,
};
use rmng_nervous::AgentRouter;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;
use tracing::info;

fn socket_path() -> PathBuf {
    rmng_core::socket_path()
}

async fn handle_line(
    line: &str,
    runtime: &Runtime,
    orchestrator: &Arc<DaemonOrchestrator>,
) -> String {
    match parse_daemon_line(line) {
        Ok(DaemonLine::OrchestrationContinue { session_id }) => {
            let resp = orchestrator.continue_session(&session_id).await;
            serde_json::to_string(&resp).unwrap_or_else(|e| {
                serde_json::to_string(&OrchestrationContinueResponse::failure(
                    &session_id,
                    e.to_string(),
                ))
                .unwrap()
            })
        }
        Ok(DaemonLine::Intent(incoming)) => {
            let session_id = extract_session_id(&incoming);
            let core_intent = match &incoming {
                IncomingIntent::Core(i) => Some(i.clone()),
                _ => None,
            };
            match runtime.handle_incoming(&incoming).await {
                Ok(resp) => {
                    if let (Some(sid), Some(intent)) = (session_id.as_deref(), core_intent.as_ref())
                    {
                        if orchestrator.should_trigger_continue(sid, intent, &resp) {
                            let orch = Arc::clone(orchestrator);
                            let sid = sid.to_string();
                            let intent = intent.clone();
                            let resp_clone = resp.clone();
                            tokio::spawn(async move {
                                if let Some(cont) = orch
                                    .maybe_continue_after_dispatch(&sid, &intent, &resp_clone)
                                    .await
                                {
                                    info!(
                                        session = %sid,
                                        steps = cont.steps_run,
                                        status = %cont.status,
                                        "daemon background auto-continue finished"
                                    );
                                }
                            });
                        }
                    }
                    serde_json::to_string(&resp).unwrap_or_else(|e| {
                        serde_json::to_string(&HandleResponse::failure(e.to_string())).unwrap()
                    })
                }
                Err(e) => serde_json::to_string(&HandleResponse::failure(e.to_string())).unwrap(),
            }
        }
        Err(e) => serde_json::to_string(&HandleResponse::failure(e.to_string())).unwrap(),
    }
}

fn extract_session_id(incoming: &IncomingIntent) -> Option<String> {
    match incoming {
        IncomingIntent::Core(intent) => intent
            .metadata()
            .and_then(|m| m.session_id.clone()),
        _ => None,
    }
}

async fn handle_connection(
    stream: tokio::net::UnixStream,
    runtime: Arc<Runtime>,
    orchestrator: Arc<DaemonOrchestrator>,
) {
    let (reader, mut writer) = stream.into_split();
    let mut lines = BufReader::new(reader).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        if line.trim().is_empty() {
            continue;
        }
        let response = handle_line(&line, &runtime, &orchestrator).await;
        if writer
            .write_all(format!("{response}
").as_bytes())
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
    info!(socket = %path.display(), "rmngd listening (orchestration auto-continue enabled)");

    let runtime = Arc::new(
        Runtime::bootstrap().unwrap_or_else(|e| {
            tracing::warn!(error = %e, "bootstrap failed — starting in degraded mode");
            Runtime::default()
        }),
    );
    let router = AgentRouter::load();
    let orchestrator = Arc::new(DaemonOrchestrator::new(
        (*runtime).clone(),
        router,
    ));

    loop {
        let (stream, _) = listener.accept().await.expect("accept");
        let runtime = runtime.clone();
        let orchestrator = orchestrator.clone();
        tokio::spawn(handle_connection(stream, runtime, orchestrator));
    }
}
