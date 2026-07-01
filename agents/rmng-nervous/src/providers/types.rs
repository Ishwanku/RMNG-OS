use rmng_core::CoreIntent;

/// Session/agent context passed to every provider adapter.
#[derive(Debug, Clone, Default)]
pub struct LlmReasonContext<'a> {
    pub session_id: Option<&'a str>,
    pub agent_id: Option<&'a str>,
    pub skill_name: Option<&'a str>,
}

/// Standard nervous-system request — all providers receive the same shape.
#[derive(Debug, Clone)]
pub struct LlmRequest<'a> {
    pub assembled_prompt: &'a str,
    pub ctx: LlmReasonContext<'a>,
}

/// Standard nervous-system response — raw JSON text from the model.
#[derive(Debug, Clone)]
pub struct LlmResponse {
    pub content: String,
    pub provider_id: &'static str,
    pub model: String,
}

/// Classified failure kind for health/matrix reporting (Sprint 7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderErrorKind {
    Misconfigured,
    InvalidKey,
    Billing,
    ModelNotFound,
    RateLimit,
    Network,
    InvalidIntent,
    Other,
}

#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("provider misconfigured: {0}")]
    Misconfigured(String),
    #[error("{provider} API error ({status}): {message}")]
    Api {
        provider: String,
        status: u16,
        message: String,
    },
    #[error("invalid intent from model: {0}")]
    InvalidIntent(#[from] rmng_core::RmngError),
    #[error("provider not implemented: {0}")]
    NotImplemented(String),
}

impl ProviderError {
    pub fn kind(&self) -> ProviderErrorKind {
        match self {
            Self::Misconfigured(_) => ProviderErrorKind::Misconfigured,
            Self::InvalidIntent(_) => ProviderErrorKind::InvalidIntent,
            Self::NotImplemented(_) => ProviderErrorKind::Other,
            Self::Http(e) if e.is_timeout() || e.is_connect() => ProviderErrorKind::Network,
            Self::Http(_) => ProviderErrorKind::Network,
            Self::Api { status, message, .. } => classify_api_error(*status, message),
        }
    }

    pub fn api(provider: &str, status: u16, message: &str) -> Self {
        let hint = match classify_api_error(status, message) {
            ProviderErrorKind::InvalidKey => " — invalid or expired API key",
            ProviderErrorKind::Billing => {
                " — billing/credits issue (add credits or check account status)"
            }
            ProviderErrorKind::ModelNotFound => " — model not found or not enabled for this key",
            ProviderErrorKind::RateLimit => " — rate limited; retry with backoff",
            _ => "",
        };
        let trimmed = if message.len() > 240 {
            format!("{}…", &message[..240])
        } else {
            message.to_string()
        };
        Self::Api {
            provider: provider.to_string(),
            status,
            message: format!("{trimmed}{hint}"),
        }
    }

    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Http(e) => e.is_timeout() || e.is_connect() || e.is_request(),
            Self::Api { status, .. } => matches!(*status, 429 | 500 | 502 | 503 | 504),
            Self::InvalidIntent(_) => true,
            _ => false,
        }
    }

    /// Whether the nervous layer should try the next profile in `llm_fallback` (Sprint 8).
    pub fn warrants_provider_fallback(&self) -> bool {
        matches!(
            self.kind(),
            ProviderErrorKind::RateLimit
                | ProviderErrorKind::Network
                | ProviderErrorKind::Billing
                | ProviderErrorKind::ModelNotFound
                | ProviderErrorKind::Other
        )
    }
}

fn classify_api_error(status: u16, message: &str) -> ProviderErrorKind {
    let lower = message.to_lowercase();
    if status == 401 {
        return ProviderErrorKind::InvalidKey;
    }
    if status == 429 {
        return ProviderErrorKind::RateLimit;
    }
    if status == 404 && (lower.contains("model") || lower.contains("not found")) {
        return ProviderErrorKind::ModelNotFound;
    }
    if status == 403 || status == 402 {
        if lower.contains("credit")
            || lower.contains("billing")
            || lower.contains("quota")
            || lower.contains("payment")
            || lower.contains("license")
            || lower.contains("spend")
        {
            return ProviderErrorKind::Billing;
        }
        return ProviderErrorKind::InvalidKey;
    }
    if lower.contains("model") && (lower.contains("not found") || lower.contains("does not exist")) {
        return ProviderErrorKind::ModelNotFound;
    }
    ProviderErrorKind::Other
}

/// Parse provider output into a v2 CoreIntent.
pub fn parse_core_intent(content: &str) -> Result<CoreIntent, ProviderError> {
    let trimmed = content.trim();
    // Strip markdown fences if the model wrapped JSON
    let json = if trimmed.starts_with("```") {
        trimmed
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim()
    } else {
        trimmed
    };
    CoreIntent::parse(json).map_err(ProviderError::InvalidIntent)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn warrants_fallback_on_rate_limit_and_billing() {
        let rate = ProviderError::api("groq", 429, "rate limit exceeded");
        assert!(rate.warrants_provider_fallback());
        assert_eq!(rate.kind(), ProviderErrorKind::RateLimit);

        let billing = ProviderError::api("grok", 403, "insufficient credits for billing");
        assert!(billing.warrants_provider_fallback());
        assert_eq!(billing.kind(), ProviderErrorKind::Billing);
    }

    #[test]
    fn invalid_key_does_not_warrant_fallback() {
        let key = ProviderError::api("openai", 401, "invalid api key");
        assert!(!key.warrants_provider_fallback());
        assert_eq!(key.kind(), ProviderErrorKind::InvalidKey);
    }
}