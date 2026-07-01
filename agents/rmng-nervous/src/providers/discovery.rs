use super::catalog::list_catalog_models;
use super::defaults::default_endpoint;
use super::factory::LlmBackend;
use super::types::ProviderError;
use rmng_core::{LlmConfig, LlmProvider};
use std::collections::HashSet;

/// Result of comparing provider API models against the local catalog.
#[derive(Debug, Clone)]
pub struct ModelSyncReport {
    pub provider: LlmProvider,
    pub live_models: Vec<String>,
    pub catalog_only: Vec<String>,
    pub live_only: Vec<String>,
    pub detail: Option<String>,
}

/// Fetch model ids from the provider API (when supported).
pub async fn fetch_live_models(cfg: &LlmConfig) -> Result<Vec<String>, ProviderError> {
    if cfg.is_mock() {
        return Ok(Vec::new());
    }

    match cfg.llm_provider {
        LlmProvider::Ollama => fetch_ollama_models(cfg).await,
        LlmProvider::Google => fetch_google_models(cfg).await,
        LlmProvider::Anthropic => Ok(Vec::new()),
        LlmProvider::OpenAi
        | LlmProvider::Grok
        | LlmProvider::Groq
        | LlmProvider::Together
        | LlmProvider::Fireworks
        | LlmProvider::DeepSeek
        | LlmProvider::NvidiaNim
        | LlmProvider::Custom => fetch_openai_compat_models(cfg).await,
        LlmProvider::None => Ok(Vec::new()),
    }
}

/// Compare live API models with catalog entries; surfaces drift warnings.
pub async fn compare_models(
    provider: LlmProvider,
    include_specialized: bool,
) -> Result<ModelSyncReport, ProviderError> {
    let cfg = LlmConfig {
        llm_provider: provider,
        endpoint_url: default_endpoint(provider),
        api_key_env_var: super::defaults::default_api_key_env(provider),
        ..Default::default()
    };

    if provider == LlmProvider::Anthropic {
        return Ok(ModelSyncReport {
            provider,
            live_models: vec![],
            catalog_only: catalog_ids(provider, include_specialized),
            live_only: vec![],
            detail: Some(
                "Anthropic has no public /models API — compare catalog manually".into(),
            ),
        });
    }

    let live = fetch_live_models(&cfg).await?;

    let catalog = catalog_ids(provider, include_specialized);
    let live_set: HashSet<&str> = live.iter().map(|s| s.as_str()).collect();
    let catalog_set: HashSet<&str> = catalog.iter().map(|s| s.as_str()).collect();

    let catalog_only: Vec<String> = catalog
        .iter()
        .filter(|id| !live_set.contains(id.as_str()))
        .cloned()
        .collect();
    let live_only: Vec<String> = live
        .iter()
        .filter(|id| !catalog_set.contains(id.as_str()))
        .cloned()
        .collect();

    Ok(ModelSyncReport {
        provider,
        live_models: live,
        catalog_only,
        live_only,
        detail: None,
    })
}

fn catalog_ids(provider: LlmProvider, include_specialized: bool) -> Vec<String> {
    list_catalog_models(provider, include_specialized)
        .into_iter()
        .map(|m| m.id)
        .collect()
}

async fn fetch_openai_compat_models(cfg: &LlmConfig) -> Result<Vec<String>, ProviderError> {
    let backend = LlmBackend::from_config(cfg)?;
    let Some(LlmBackend::OpenAiCompat(p)) = backend else {
        return Ok(Vec::new());
    };
    p.list_models().await
}

async fn fetch_ollama_models(cfg: &LlmConfig) -> Result<Vec<String>, ProviderError> {
    let backend = LlmBackend::from_config(cfg)?;
    let Some(LlmBackend::Ollama(p)) = backend else {
        return Ok(Vec::new());
    };
    p.list_models().await
}

async fn fetch_google_models(cfg: &LlmConfig) -> Result<Vec<String>, ProviderError> {
    let backend = LlmBackend::from_config(cfg)?;
    let Some(LlmBackend::Google(p)) = backend else {
        return Ok(Vec::new());
    };
    p.list_models().await
}



