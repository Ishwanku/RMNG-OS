//! Sprint 29 — `rmngd --validate` integration tests.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, MutexGuard};

static ENV_TEST_LOCK: Mutex<()> = Mutex::new(());

fn env_test_guard() -> MutexGuard<'static, ()> {
    ENV_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

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

fn isolated_home() -> PathBuf {
    std::env::temp_dir().join(format!("rmng-validate-e2e-{}", uuid::Uuid::new_v4()))
}

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("project root")
}

#[test]
fn validate_exits_nonzero_on_invalid_config() {
    let _lock = env_test_guard();
    let home = isolated_home();
    let _guard = HomeGuard::set(&home);
    std::fs::create_dir_all(home.join(".rmng")).expect("mkdir");
    std::fs::write(home.join(".rmng/config.toml"), "not-valid-toml [[[")
        .expect("bad config");

    let bin = env!("CARGO_BIN_EXE_rmngd");
    let out = Command::new(bin)
        .arg("--validate")
        .env("HOME", &home)
        .env("RMNG_PROJECT_ROOT", project_root())
        .output()
        .expect("spawn");
    assert!(!out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("ERROR") || stdout.contains("not ready"));
}

#[test]
fn validate_human_output_lists_checks() {
    let _lock = env_test_guard();
    let home = isolated_home();
    let _guard = HomeGuard::set(&home);
    std::fs::create_dir_all(home.join(".rmng")).expect("mkdir");

    let bin = env!("CARGO_BIN_EXE_rmngd");
    let out = Command::new(bin)
        .arg("--validate")
        .env("HOME", &home)
        .env("RMNG_PROJECT_ROOT", project_root())
        .output()
        .expect("spawn");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("rmngd --validate"));
    assert!(stdout.contains("config"));
    assert!(stdout.contains("agents"));
}