//! Sprint 27 — `orchestration.continue` over the real Unix socket.

use rmng_core::{
    daemon_running, parse_daemon_line, send_intent_json, DaemonLine, OrchestrationContinueResponse,
    SessionStore,
};
use std::path::{Path, PathBuf};
use std::time::Duration;
use std::sync::{Mutex, MutexGuard};
use tokio::time::sleep;
/// Both tests mutate process-global HOME; serialize to avoid cross-test races.
static ENV_TEST_LOCK: Mutex<()> = Mutex::new(());

fn env_test_guard() -> MutexGuard<'static, ()> {
    ENV_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

fn isolated_home() -> PathBuf {
    std::env::temp_dir().join(format!("rmng-sock-e2e-{}", uuid::Uuid::new_v4()))
}

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("project root")
}

/// Point this test process at the same HOME as the spawned rmngd.
struct HomeGuard {
    prev: Option<String>,
}

impl HomeGuard {
    fn set(home: &Path) -> Self {
        let prev = std::env::var("HOME").ok();
        std::env::set_var("HOME", home);
        Self { prev }
    }
}

impl Drop for HomeGuard {
    fn drop(&mut self) {
        if let Some(ref p) = self.prev {
            std::env::set_var("HOME", p);
        }
    }
}

async fn wait_for_socket(max_ms: u64) -> bool {
    let step = Duration::from_millis(50);
    let mut waited = 0u64;
    while waited < max_ms {
        if daemon_running() {
            return true;
        }
        sleep(step).await;
        waited += 50;
    }
    false
}

#[tokio::test]
async fn orchestration_continue_over_unix_socket() {
    let _env_lock = env_test_guard();
    if !cfg!(unix) {
        return;
    }

    let home = isolated_home();
    let _home_guard = HomeGuard::set(&home);
    std::fs::create_dir_all(home.join(".rmng")).expect("mkdir");
    let sessions = SessionStore::new(&home.join(".rmng/sessions"));
    let session = sessions.create().expect("create");
    {
        let mut loaded = sessions.load(&session.id).expect("load");
        sessions
            .set_orchestration_state(
                &mut loaded,
                serde_json::json!({
                    "status": "completed",
                    "awaiting_continuation": true,
                    "continuation_agent": "swarm-coordinator",
                }),
            )
            .expect("orch");
    }

    let root = project_root();
    let bin = env!("CARGO_BIN_EXE_rmngd");
    let mut child = tokio::process::Command::new(bin)
        .current_dir(&root)
        .env("HOME", &home)
        .env("RMNG_PROJECT_ROOT", &root)
        .env("RUST_LOG", "warn")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("spawn rmngd");

    assert!(
        wait_for_socket(3000).await,
        "rmngd socket did not appear"
    );

    let req = format!(
        r#"{{"action":"orchestration.continue","session_id":"{}"}}"#,
        session.id
    );
    let line = send_intent_json(&req).await.expect("ipc");
    let parsed = parse_daemon_line(&req).expect("parse req");
    assert!(matches!(parsed, DaemonLine::OrchestrationContinue { .. }));

    let resp: OrchestrationContinueResponse =
        serde_json::from_str(line.trim()).expect("parse response");
    assert_eq!(resp.session_id, session.id);
    assert_eq!(resp.action, "orchestration.continue");
    assert!(resp.ok, "continue failed: {:?}", resp.error);

    let _ = child.kill().await;
    let _ = std::fs::remove_dir_all(&home);
}

#[tokio::test]
async fn socket_rejects_invalid_continue_without_session_id() {
    let _env_lock = env_test_guard();
    if !cfg!(unix) {
        return;
    }

    let home = isolated_home();
    let _home_guard = HomeGuard::set(&home);
    std::fs::create_dir_all(home.join(".rmng")).expect("mkdir");

    let root = project_root();
    let bin = env!("CARGO_BIN_EXE_rmngd");
    let mut child = tokio::process::Command::new(bin)
        .current_dir(&root)
        .env("HOME", &home)
        .env("RMNG_PROJECT_ROOT", &root)
        .spawn()
        .expect("spawn");

    assert!(wait_for_socket(3000).await);

    let bad = r#"{"action":"orchestration.continue"}"#;
    assert!(parse_daemon_line(bad).is_err());

    let _ = child.kill().await;
    let _ = std::fs::remove_dir_all(&home);
}
