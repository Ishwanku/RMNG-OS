use super::anthropic::AnthropicProvider;
use super::defaults::{default_endpoint, default_model, provider_label, resolve_api_key};
use super::google::GoogleProvider;
use super::ollama::OllamaProvider;
use super::openai_compat::OpenAiCompatProvider;
use super::types::{LlmReasonContext, ProviderError};
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

    pub async fn reason_core(
        &self,
        assembled: &str,
        ctx: &LlmReasonContext<'_>,
    ) -> Result<CoreIntent, ProviderError> {
        match self {
            Self::Ollama(p) => p.reason_core(assembled, ctx).await,
            Self::OpenAiCompat(p) => p.reason_core(assembled, ctx).await,
            Self::Anthropic(p) => p.reason_core(assembled, ctx).await,
            Self::Google(p) => p.reason_core(assembled, ctx).await,
        }
    }
}

pub async fn health_check(cfg: &RmngConfig) -> Result<(String, bool, Option<String>), ProviderError> {
    let label = provider_label(cfg.llm.llm_provider);
    if cfg.llm.is_mock() {
        return Ok((label.to_string(), true, Some("mock — no network".into())));
    }
    let backend = LlmBackend::from_config(&cfg.llm)?;
    match backend {
        Some(b) => {
            let ok = b.health().await.unwrap_or(false);
            let detail = if ok {
                Some(format!("model={}", cfg.llm.model.as_deref().unwrap_or("default")))
            } else {
                Some("health probe failed".into())
            };
            Ok((b.id().to_string(), ok, detail))
        }
        None => Ok((label.to_string(), true, Some("mock".into()))),
    }
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