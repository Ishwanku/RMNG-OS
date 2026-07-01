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
    pub fn api(provider: &str, status: u16, message: &str) -> Self {
        let hint = match status {
            401 => " — invalid or expired API key",
            403 => " — access denied (add credits/licenses at console.x.ai or check model access)",
            429 => " — rate limited; retry with backoff",
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
            _ => false,
        }
    }
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