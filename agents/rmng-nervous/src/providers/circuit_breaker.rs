use super::types::ProviderErrorKind;
use crate::nervous_audit::log_nervous_event;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
struct CircuitState {
    failures: u32,
    open_until: Option<Instant>,
}

static BREAKERS: Mutex<Option<HashMap<String, CircuitState>>> = Mutex::new(None);

fn map() -> std::sync::MutexGuard<'static, Option<HashMap<String, CircuitState>>> {
    let mut guard = BREAKERS.lock().expect("circuit breaker lock");
    if guard.is_none() {
        *guard = Some(HashMap::new());
    }
    guard
}

/// Whether a provider request should be attempted (not circuit-open).
pub fn allow_request(provider_id: &str) -> bool {
    let mut guard = map();
    let Some(states) = guard.as_mut() else {
        return true;
    };
    let state = states.entry(provider_id.to_string()).or_insert(CircuitState {
        failures: 0,
        open_until: None,
    });
    if let Some(until) = state.open_until {
        if Instant::now() < until {
            return false;
        }
        state.open_until = None;
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
        if state.failures > 0 || state.open_until.is_some() {
            log_nervous_event(
                "nervous.circuit_breaker",
                "closed",
                Some(&format!("provider={provider_id} recovered")),
            );
        }
        state.failures = 0;
        state.open_until = None;
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
    let state = states.entry(provider_id.to_string()).or_insert(CircuitState {
        failures: 0,
        open_until: None,
    });
    state.failures = state.failures.saturating_add(1);
    let secs = (30u64).saturating_mul(1u64 << state.failures.min(4));
    let cooldown = Duration::from_secs(secs.min(300));
    state.open_until = Some(Instant::now() + cooldown);
    log_nervous_event(
        "nervous.circuit_breaker",
        "open",
        Some(&format!(
            "provider={provider_id} kind={kind:?} failures={} cooldown_secs={}",
            state.failures,
            cooldown.as_secs()
        )),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opens_after_rate_limit_failure() {
        let id = format!("test-provider-{}", uuid::Uuid::new_v4());
        assert!(allow_request(&id));
        record_failure(&id, ProviderErrorKind::RateLimit);
        assert!(!allow_request(&id));
        record_success(&id);
        assert!(allow_request(&id));
    }
}