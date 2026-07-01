use super::anthropic::AnthropicProvider;
use super::defaults::{default_endpoint, default_model, provider_label, resolve_api_key};
use super::google::GoogleProvider;
use super::ollama::OllamaProvider;
use super::openai_compat::OpenAiCompatProvider;
use super::reason::reason_with_retry;
use super::types::{LlmReasonContext, LlmRequest, LlmResponse, ProviderError};
use rmng_core::{CoreIntent, LlmConfig, LlmProvider, RmngConfig};

/// Unified LLM backend — dispatches to the correct adapter.
pub enum LlmBackend {
    Ollama(OllamaProvider),
    OpenAiCompat(OpenAiCompatProvider),
    Anthropic(AnthropicProvider),
    Google(GoogleProvider),
}

impl LlmBackend {
    pub fn from_config(cfg: &LlmConfig) -> Result<Option<Self>, ProviderError> {
        if cfg.is_mock() {
            return Ok(None);
        }
        let endpoint = cfg
            .endpoint_url
            .clone()
            .or_else(|| default_endpoint(cfg.llm_provider).map(str::to_string));
        let model = cfg
            .model
            .clone()
            .unwrap_or_else(|| default_model(cfg.llm_provider).to_string());
        let timeout = cfg.timeout_secs;
        let retries = cfg.max_retries;

        match cfg.llm_provider {
            LlmProvider::Ollama => {
                let url = endpoint.ok_or_else(|| {
                    ProviderError::Misconfigured("ollama endpoint_url required".into())
                })?;
                Ok(Some(Self::Ollama(OllamaProvider::new(
                    url, model, timeout, retries,
                ))))
            }
            LlmProvider::OpenAi
            | LlmProvider::Grok
            | LlmProvider::Groq
            | LlmProvider::Together
            | LlmProvider::Fireworks
            | LlmProvider::DeepSeek
            | LlmProvider::NvidiaNim
            | LlmProvider::Custom => {
                let url = endpoint.ok_or_else(|| {
                    ProviderError::Misconfigured(format!(
                        "{} endpoint_url required (or use provider default)",
                        provider_label(cfg.llm_provider)
                    ))
                })?;
                let api_key = resolve_api_key(cfg)
                    .map_err(ProviderError::Misconfigured)?
                    .ok_or_else(|| {
                        ProviderError::Misconfigured(format!(
                            "{} API key missing — set {} or api_key in config",
                            provider_label(cfg.llm_provider),
                            cfg.api_key_env_var.as_deref().unwrap_or("env var")
                        ))
                    })?;
                let id = provider_label(cfg.llm_provider);
                Ok(Some(Self::OpenAiCompat(OpenAiCompatProvider::new(
                    id, url, api_key, model, timeout, retries,
                ))))
            }
            LlmProvider::Anthropic => {
                let url = endpoint.unwrap_or_else(|| {
                    default_endpoint(LlmProvider::Anthropic)
                        .unwrap()
                        .to_string()
                });
                let api_key = resolve_api_key(cfg)
                    .map_err(ProviderError::Misconfigured)?
                    .ok_or_else(|| {
                        ProviderError::Misconfigured(
                            "anthropic API key missing — set ANTHROPIC_API_KEY".into(),
                        )
                    })?;
                Ok(Some(Self::Anthropic(AnthropicProvider::new(
                    url, api_key, model, timeout, retries,
                ))))
            }
            LlmProvider::Google => {
                let url = endpoint.unwrap_or_else(|| {
                    default_endpoint(LlmProvider::Google)
                        .unwrap()
                        .to_string()
                });
                let api_key = resolve_api_key(cfg)
                    .map_err(ProviderError::Misconfigured)?
                    .ok_or_else(|| {
                        ProviderError::Misconfigured(
                            "google API key missing — set GOOGLE_API_KEY".into(),
                        )
                    })?;
                Ok(Some(Self::Google(GoogleProvider::new(
                    url, api_key, model, timeout, retries,
                ))))
            }
            LlmProvider::None => Ok(None),
        }
    }

    pub fn id(&self) -> &'static str {
        match self {
            Self::Ollama(p) => p.id(),
            Self::OpenAiCompat(p) => p.id(),
            Self::Anthropic(p) => p.id(),
            Self::Google(p) => p.id(),
        }
    }

    pub async fn health(&self) -> Result<bool, ProviderError> {
        match self {
            Self::Ollama(p) => p.health().await,
            Self::OpenAiCompat(p) => p.health().await,
            Self::Anthropic(p) => p.health().await,
            Self::Google(p) => p.health().await,
        }
    }

    pub async fn complete(&self, req: LlmRequest<'_>) -> Result<LlmResponse, ProviderError> {
        match self {
            Self::Ollama(p) => p.complete(req).await,
            Self::OpenAiCompat(p) => p.complete(req).await,
            Self::Anthropic(p) => p.complete(req).await,
            Self::Google(p) => p.complete(req).await,
        }
    }

    pub async fn reason_core(
        &self,
        assembled: &str,
        ctx: &LlmReasonContext<'_>,
    ) -> Result<CoreIntent, ProviderError> {
        reason_with_retry(self, self.id(), assembled, ctx).await
    }
}

/// Detailed health probe result for CLI/observe.
#[derive(Debug, Clone)]
pub struct HealthReport {
    pub provider_id: String,
    pub healthy: bool,
    pub model: String,
    pub endpoint: Option<String>,
    pub api_key_set: bool,
    pub detail: String,
}

pub async fn health_check_detailed(cfg: &RmngConfig) -> Result<HealthReport, ProviderError> {
    let label = provider_label(cfg.llm.llm_provider);
    let model = cfg
        .llm
        .model
        .clone()
        .unwrap_or_else(|| default_model(cfg.llm.llm_provider).to_string());
    let endpoint = cfg
        .llm
        .endpoint_url
        .clone()
        .or_else(|| default_endpoint(cfg.llm.llm_provider).map(str::to_string));
    let api_key_set = super::defaults::resolve_api_key(&cfg.llm)
        .ok()
        .flatten()
        .is_some();

    if cfg.llm.is_mock() {
        return Ok(HealthReport {
            provider_id: label.to_string(),
            healthy: true,
            model,
            endpoint,
            api_key_set: false,
            detail: "mock — no network".into(),
        });
    }

    if !api_key_set && cfg.llm.llm_provider != LlmProvider::Ollama {
        return Ok(HealthReport {
            provider_id: label.to_string(),
            healthy: false,
            model,
            endpoint,
            api_key_set: false,
            detail: format!(
                "API key missing — export {} or set api_key_env_var in config",
                cfg.llm
                    .api_key_env_var
                    .as_deref()
                    .or_else(|| super::defaults::default_api_key_env(cfg.llm.llm_provider))
                    .unwrap_or("RMNG_LLM_API_KEY")
            ),
        });
    }

    let backend = LlmBackend::from_config(&cfg.llm)?;
    match backend {
        Some(b) => {
            let (healthy, detail) = match b.health().await {
                Ok(true) if b.id() == "anthropic" => (
                    true,
                    "API key set (live probe skipped to save tokens — run scripts/probe-anthropic-minimal.py)".into(),
                ),
                Ok(true) => (true, "endpoint reachable".into()),
                Ok(false) => (false, "health probe returned false".into()),
                Err(ProviderError::Api { status, message, .. }) => {
                    (false, format!("API {status}: {message}"))
                }
                Err(e) => (false, format!("health probe failed: {e}")),
            };
            Ok(HealthReport {
                provider_id: b.id().to_string(),
                healthy,
                model,
                endpoint,
                api_key_set,
                detail,
            })
        }
        None => Ok(HealthReport {
            provider_id: label.to_string(),
            healthy: true,
            model,
            endpoint,
            api_key_set: false,
            detail: "mock".into(),
        }),
    }
}

pub async fn health_check(cfg: &RmngConfig) -> Result<(String, bool, Option<String>), ProviderError> {
    let r = health_check_detailed(cfg).await?;
    Ok((
        r.provider_id,
        r.healthy,
        Some(format!(
            "{} model={} key_set={}",
            r.detail, r.model, r.api_key_set
        )),
    ))
}

pub fn list_supported_providers() -> Vec<(&'static str, &'static str, bool)> {
    vec![
        ("none", "Mock intents (default, no network)", true),
        ("ollama", "Local Ollama /api/generate", true),
        ("openai", "OpenAI GPT-4o / o1 (API)", true),
        ("grok", "xAI Grok (OpenAI-compatible API)", true),
        ("anthropic", "Anthropic Claude (Messages API)", true),
        ("google", "Google Gemini (generateContent)", true),
        ("groq", "Groq (OpenAI-compatible)", true),
        ("together", "Together AI (OpenAI-compatible)", true),
        ("fireworks", "Fireworks AI (OpenAI-compatible)", true),
        ("deepseek", "DeepSeek (OpenAI-compatible)", true),
        ("nvidia_nim", "NVIDIA NIM (OpenAI-compatible)", true),
        ("custom", "Self-hosted OpenAI-compatible (vLLM, etc.)", true),
    ]
}