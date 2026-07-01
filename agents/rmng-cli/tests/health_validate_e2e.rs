//! Sprint 29 — integration tests for `rmng health` and `rmngd --validate`.

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
    std::env::temp_dir().join(format!("rmng-health-e2e-{}", uuid::Uuid::new_v4()))
}

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("project root")
}


fn bin_exe(name: &str) -> PathBuf {
    std::env::var(format!("CARGO_BIN_EXE_{name}")).map(PathBuf::from).unwrap_or_else(|_| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("target")
            .join(std::env::var("PROFILE").unwrap_or_else(|_| "debug".to_string()))
            .join(name)
    })
}

fn run_rmng(home: &Path, args: &[&str]) -> (i32, String) {
    let bin = env!("CARGO_BIN_EXE_rmng");
    let out = Command::new(bin)
        .args(args)
        .env("HOME", home)
        .env("RMNG_PROJECT_ROOT", project_root())
        .output()
        .expect("spawn rmng");
    let code = out.status.code().unwrap_or(127);
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    (code, format!("{stdout}{stderr}"))
}

fn run_rmngd_validate(home: &Path, project_root: Option<&Path>) -> (i32, String) {
    let bin = bin_exe("rmngd");
    let mut cmd = Command::new(bin);
    cmd.arg("--validate").env("HOME", home);
    if let Some(root) = project_root {
        cmd.env("RMNG_PROJECT_ROOT", root);
    } else {
        cmd.env_remove("RMNG_PROJECT_ROOT");
    }
    let out = cmd.output().expect("spawn rmngd");
    let code = out.status.code().unwrap_or(127);
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    (code, stdout)
}

#[test]
fn health_json_default_allows_stopped_daemon() {
    let _lock = env_test_guard();
    let home = isolated_home();
    let _guard = HomeGuard::set(&home);
    std::fs::create_dir_all(home.join(".rmng")).expect("mkdir");

    let (code, out) = run_rmng(&home, &["health", "--json", "--quick"]);
    assert_eq!(code, 0, "stdout: {out}");
    let v: serde_json::Value = serde_json::from_str(&out).expect("json");
    assert_eq!(v["schema_version"], 2);
    assert_eq!(v["rmngd_running"], false);
    assert_eq!(v["ok"], true);
    assert!(v["failures"].as_array().unwrap().is_empty());
}

#[test]
fn health_require_daemon_fails_when_stopped() {
    let _lock = env_test_guard();
    let home = isolated_home();
    let _guard = HomeGuard::set(&home);
    std::fs::create_dir_all(home.join(".rmng")).expect("mkdir");

    let (code, out) = run_rmng(&home, &["health", "--json", "--quick", "--require-daemon"]);
    assert_eq!(code, 1, "stdout: {out}");
    let v: serde_json::Value = serde_json::from_str(&out).expect("json");
    assert_eq!(v["ok"], false);
    let failures: Vec<String> = v["failures"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_str().unwrap().to_string())
        .collect();
    assert!(failures.iter().any(|f| f.contains("rmngd")));
}

#[test]
fn health_strict_fails_on_open_circuits() {
    let _lock = env_test_guard();
    let home = isolated_home();
    let _guard = HomeGuard::set(&home);
    std::fs::create_dir_all(home.join(".rmng")).expect("mkdir");
    let circuit_state = home.join(".rmng/circuit-state.json");
    std::fs::write(
        &circuit_state,
        r#"{"version":2,"last_updated_unix":0,"providers":{"grok":{"failures":3,"open_until_unix":9999999999}}}"#,
    )
    .expect("circuit state");

    let (code, out) = run_rmng(&home, &["health", "--json", "--quick", "--strict"]);
    assert_eq!(code, 1, "stdout: {out}");
    let v: serde_json::Value = serde_json::from_str(&out).expect("json");
    assert_eq!(v["ok"], false);
    assert!(v["circuits_open"].as_u64().unwrap_or(0) > 0);
}

#[test]
fn rmngd_validate_fails_without_project_root() {
    let _lock = env_test_guard();
    let home = isolated_home();
    let _guard = HomeGuard::set(&home);
    std::fs::create_dir_all(home.join(".rmng")).expect("mkdir");

    let (code, out) = run_rmngd_validate(&home, None);
    assert_eq!(code, 1, "stdout: {out}");
    assert!(out.contains("not ready") || out.contains("ERROR"));
}

#[test]
fn rmngd_validate_passes_with_project_root() {
    let _lock = env_test_guard();
    let home = isolated_home();
    let _guard = HomeGuard::set(&home);
    std::fs::create_dir_all(home.join(".rmng")).expect("mkdir");

    let root = project_root();
    let (code, out) = run_rmngd_validate(&home, Some(&root));
    assert_eq!(code, 0, "stdout: {out}");
    assert!(out.contains("ready"));
}