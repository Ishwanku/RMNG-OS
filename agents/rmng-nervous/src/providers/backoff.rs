use std::time::Duration;

/// Exponential backoff with jitter (Sprint 9). Base 400ms, cap 30s.
pub fn retry_delay(attempt: u32) -> Duration {
    let base_ms = 400u64.saturating_mul(1u64 << attempt.min(6));
    let cap_ms = 30_000u64;
    let capped = base_ms.min(cap_ms);
    // Simple jitter: ±25% via attempt-derived offset (no rand dep)
    let jitter = (attempt as u64 * 137) % (capped / 4 + 1);
    Duration::from_millis(capped.saturating_sub(capped / 8).saturating_add(jitter))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_grows_with_attempt() {
        assert!(retry_delay(0) < retry_delay(3));
        assert!(retry_delay(3) <= Duration::from_secs(30));
    }
}