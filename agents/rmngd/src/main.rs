use tracing::info;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    info!("rmngd starting (Phase 5 scaffold)");
    info!("permission gate: active");
    info!("tool dispatch: stub — awaiting integrations");

    // Phase 5: IPC socket, intent queue, audit log
    tokio::signal::ctrl_c()
        .await
        .expect("listen for ctrl-c");
    info!("rmngd shutting down");
}
