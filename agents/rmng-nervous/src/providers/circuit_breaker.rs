use super::types::ProviderErrorKind;
use crate::nervous_audit::log_nervous_event;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const STATE_VERSION: u32 = 2;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedCircuitState {
    failures: u32,
    open_until_unix: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct CircuitStateFile {
    #[serde(default = "default_version")]
    version: u32,
    #[serde(default)]
    last_updated_unix: u64,
    #[serde(default)]
    providers: HashMap<String, PersistedCircuitState>,
}

fn default_version() -> u32 {
    STATE_VERSION
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitStatus {
    pub provider_id: String,
    pub failures: u32,
    pub open: bool,
    pub open_until_unix: Option<u64>,
    pub cooldown_secs_remaining: Option<u64>,
}

static BREAKERS: Mutex<Option<HashMap<String, PersistedCircuitState>>> = Mutex::new(None);
static LAST_LOAD_MTIME: Mutex<Option<u64>> = Mutex::new(None);

fn state_path() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".rmng/circuit-state.json");
    }
    PathBuf::from("/tmp/rmng/circuit-state.json")
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn file_mtime_unix(path: &PathBuf) -> Option<u64> {
    let meta = std::fs::metadata(path).ok()?;
    let modified = meta.modified().ok()?;
    modified.duration_since(UNIX_EPOCH).ok().map(|d| d.as_secs())
}

fn load_file() -> CircuitStateFile {
    let path = state_path();
    if !path.is_file() {
        return CircuitStateFile::default();
    }
    let raw = std::fs::read_to_string(&path).unwrap_or_default();
    serde_json::from_str(&raw).unwrap_or_default()
}

fn prune_empty(providers: &HashMap<String, PersistedCircuitState>) -> HashMap<String, PersistedCircuitState> {
    providers
        .iter()
        .filter(|(_, st)| st.failures > 0 || st.open_until_unix.is_some())
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

fn save_file(providers: &HashMap<String, PersistedCircuitState>) {
    let path = state_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let pruned = prune_empty(providers);
    let file = CircuitStateFile {
        version: STATE_VERSION,
        last_updated_unix: now_unix(),
        providers: pruned,
    };
    if let Ok(raw) = serde_json::to_string_pretty(&file) {
        let tmp = path.with_extension("json.tmp");
        if std::fs::write(&tmp, &raw).is_ok() {
            let _ = std::fs::rename(&tmp, &path);
            if let Ok(mut mtime) = LAST_LOAD_MTIME.lock() {
                *mtime = file_mtime_unix(&path);
            }
        }
    }
}

#[cfg(test)]
fn maybe_reload_from_disk() {}

#[cfg(not(test))]
fn maybe_reload_from_disk() {
    let path = state_path();
    let disk_mtime = file_mtime_unix(&path);
    let mut last = LAST_LOAD_MTIME.lock().expect("mtime lock");
    if disk_mtime == *last {
        return;
    }
    *last = disk_mtime;
    drop(last);
    let mut guard = BREAKERS.lock().expect("circuit breaker lock");
    *guard = Some(load_file().providers);
}

fn map() -> std::sync::MutexGuard<'static, Option<HashMap<String, PersistedCircuitState>>> {
    maybe_reload_from_disk();
    let mut guard = BREAKERS.lock().expect("circuit breaker lock");
    if guard.is_none() {
        let loaded = load_file().providers;
        *guard = Some(loaded);
        if let Ok(mut mtime) = LAST_LOAD_MTIME.lock() {
            *mtime = file_mtime_unix(&state_path());
        }
    }
    guard
}

fn persist(guard: &HashMap<String, PersistedCircuitState>) {
    save_file(guard);
}

fn is_open(state: &PersistedCircuitState) -> bool {
    state
        .open_until_unix
        .map(|u| now_unix() < u)
        .unwrap_or(false)
}

/// Whether a provider request should be attempted (not circuit-open).
pub fn allow_request(provider_id: &str) -> bool {
    let mut guard = map();
    let Some(states) = guard.as_mut() else {
        return true;
    };
    let state = states.entry(provider_id.to_string()).or_insert(PersistedCircuitState {
        failures: 0,
        open_until_unix: None,
    });
    if let Some(until) = state.open_until_unix {
        if now_unix() < until {
            return false;
        }
        state.open_until_unix = None;
        persist(states);
        log_nervous_event(
            "nervous.circuit_breaker",
            "half_open",
            Some(&format!("provider={provider_id} retrying after cooldown")),
        );
    }
    true
}

pub fn record_success(provider_id: &str) {
    let mut guard = map();
    let Some(states) = guard.as_mut() else {
        return;
    };
    if let Some(state) = states.get_mut(provider_id) {
        if state.failures > 0 || state.open_until_unix.is_some() {
            log_nervous_event(
                "nervous.circuit_breaker",
                "closed",
                Some(&format!("provider={provider_id} recovered")),
            );
        }
        state.failures = 0;
        state.open_until_unix = None;
        persist(states);
    }
}

/// Trip or extend circuit on rate-limit / billing / quota failures.
pub fn record_failure(provider_id: &str, kind: ProviderErrorKind) {
    if !matches!(
        kind,
        ProviderErrorKind::RateLimit | ProviderErrorKind::Billing | ProviderErrorKind::Other
    ) {
        return;
    }
    let mut guard = map();
    let Some(states) = guard.as_mut() else {
        return;
    };
    let (failures, cooldown_secs) = {
        let state = states.entry(provider_id.to_string()).or_insert(PersistedCircuitState {
            failures: 0,
            open_until_unix: None,
        });
        state.failures = state.failures.saturating_add(1);
        let failures = state.failures;
        let secs = (30u64).saturating_mul(1u64 << failures.min(4));
        let cooldown = Duration::from_secs(secs.min(300));
        state.open_until_unix = Some(now_unix() + cooldown.as_secs());
        (failures, cooldown.as_secs())
    };
    persist(states);
    log_nervous_event(
        "nervous.circuit_breaker",
        "open",
        Some(&format!(
            "provider={provider_id} kind={kind:?} failures={failures} cooldown_secs={cooldown_secs}"
        )),
    );
}

/// Snapshot all circuit breaker states (Sprint 11+ — for observe / health).
pub fn list_circuit_statuses() -> Vec<CircuitStatus> {
    let guard = map();
    let Some(states) = guard.as_ref() else {
        return Vec::new();
    };
    let now = now_unix();
    let mut out: Vec<CircuitStatus> = states
        .iter()
        .map(|(id, st)| {
            let open = is_open(st);
            let remaining = st.open_until_unix.and_then(|u| u.checked_sub(now));
            CircuitStatus {
                provider_id: id.clone(),
                failures: st.failures,
                open,
                open_until_unix: st.open_until_unix,
                cooldown_secs_remaining: remaining,
            }
        })
        .collect();
    out.sort_by(|a, b| a.provider_id.cmp(&b.provider_id));
    out
}

pub fn circuit_state_path() -> PathBuf {
    state_path()
}

/// Force reload from disk (tests / explicit CLI refresh).
pub fn reload_from_disk() {
    let mut guard = BREAKERS.lock().expect("circuit breaker lock");
    *guard = Some(load_file().providers);
    if let Ok(mut mtime) = LAST_LOAD_MTIME.lock() {
        *mtime = file_mtime_unix(&state_path());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn reset(home: &PathBuf) {
        env::set_var("HOME", home.to_str().unwrap());
        *BREAKERS.lock().unwrap() = None;
        *LAST_LOAD_MTIME.lock().unwrap() = None;
        let p = state_path();
        let _ = std::fs::remove_file(&p);
        let _ = std::fs::remove_file(p.with_extension("json.tmp"));
    }

    #[test]
    fn opens_after_rate_limit_failure() {
        let dir = env::temp_dir().join(format!("rmng-cb-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        reset(&dir);
        let id = format!("test-provider-{}", uuid::Uuid::new_v4());
        assert!(allow_request(&id));
        record_failure(&id, ProviderErrorKind::RateLimit);
        assert!(!allow_request(&id));
        record_success(&id);
        assert!(allow_request(&id));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn persists_across_reload_from_disk() {
        let dir = env::temp_dir().join(format!("rmng-cb-persist-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        reset(&dir);
        let id = "persist-provider".to_string();
        record_failure(&id, ProviderErrorKind::RateLimit);
        assert!(!allow_request(&id));
        assert!(state_path().is_file());
        reload_from_disk();
        assert!(!allow_request(&id));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn reloads_when_file_mtime_changes() {
        let dir = env::temp_dir().join(format!("rmng-cb-mtime-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        reset(&dir);
        let id = "mtime-provider".to_string();
        record_failure(&id, ProviderErrorKind::Billing);
        *BREAKERS.lock().unwrap() = Some(HashMap::new());
        *LAST_LOAD_MTIME.lock().unwrap() = None;
        reload_from_disk();
        assert!(!allow_request(&id));
        let _ = std::fs::remove_dir_all(dir);
    }
}
