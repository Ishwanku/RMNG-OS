//! Fallback chain runner — shared between connector and integration tests (Sprint 9).

use std::future::Future;

pub struct ChainRunResult<T> {
    pub value: T,
    pub attempt_index: usize,
    pub prior_failures: Vec<String>,
}

#[derive(Debug)]
pub struct ChainExhausted {
    pub errors: Vec<String>,
}

/// Try each attempt in order; continue on fallback-eligible errors.
pub async fn run_fallback_chain<T, E, F, Fut, FB>(
    len: usize,
    mut execute: F,
    is_fallback_eligible: FB,
) -> Result<ChainRunResult<T>, ChainExhausted>
where
    F: FnMut(usize) -> Fut,
    Fut: Future<Output = Result<T, E>>,
    FB: Fn(&E) -> bool,
    E: std::fmt::Display,
{
    let mut errors = Vec::new();
    for idx in 0..len {
        match execute(idx).await {
            Ok(value) => {
                return Ok(ChainRunResult {
                    value,
                    attempt_index: idx,
                    prior_failures: errors,
                });
            }
            Err(e) if is_fallback_eligible(&e) && idx + 1 < len => {
                errors.push(format!("attempt {idx}: {e}"));
            }
            Err(e) => {
                errors.push(format!("attempt {idx}: {e}"));
                return Err(ChainExhausted { errors });
            }
        }
    }
    Err(ChainExhausted { errors })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct SimErr(&'static str);

    impl std::fmt::Display for SimErr {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    fn rate_limit() -> SimErr {
        SimErr("rate limit")
    }

    #[tokio::test]
    async fn traverses_chain_on_retryable_errors() {
        let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let c = calls.clone();
        let result = run_fallback_chain(
            3,
            move |idx| {
                let c = c.clone();
                async move {
                    let n = c.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    if n == 0 {
                        Err(rate_limit())
                    } else if n == 1 {
                        Err(SimErr("model not found"))
                    } else {
                        Ok(idx)
                    }
                }
            },
            |_| true,
        )
        .await
        .expect("chain succeeds");
        assert_eq!(result.attempt_index, 2);
        assert_eq!(result.prior_failures.len(), 2);
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn stops_on_non_fallback_error() {
        let result = run_fallback_chain(
            2,
            |idx| async move {
                if idx == 0 {
                    Err(SimErr("invalid key"))
                } else {
                    Ok(1)
                }
            },
            |_| false,
        )
        .await;
        assert!(result.is_err());
    }
}