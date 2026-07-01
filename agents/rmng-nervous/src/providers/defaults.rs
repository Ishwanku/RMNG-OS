use super::catalog::{catalog_api_key_env, catalog_default_model, catalog_endpoint};
use rmng_core::{LlmConfig, LlmProvider};

fn static_endpoint(provider: LlmProvider) -> Option<&'static str> {
    match provider {
        LlmProvider::Ollama => Some("http://127.0.0.1:11434"),
        LlmProvider::OpenAi => Some("https://api.openai.com/v1"),
        LlmProvider::Grok => Some("https://api.x.ai/v1"),
        LlmProvider::Anthropic => Some("https://api.anthropic.com"),
        LlmProvider::Google => Some("https://generativelanguage.googleapis.com"),
        LlmProvider::Groq => Some("https://api.groq.com/openai/v1"),
        LlmProvider::Together => Some("https://api.together.xyz/v1"),
        LlmProvider::Fireworks => Some("https://api.fireworks.ai/inference/v1"),
        LlmProvider::DeepSeek => Some("https://api.deepseek.com/v1"),
        LlmProvider::NvidiaNim => Some("https://integrate.api.nvidia.com/v1"),
        LlmProvider::Custom => None,
        LlmProvider::None => None,
    }
}

fn static_model(provider: LlmProvider) -> &'static str {
    match provider {
        LlmProvider::Ollama => "llama3.2",
        LlmProvider::OpenAi => "gpt-4o",
        LlmProvider::Grok => "grok-4.3",
        LlmProvider::Anthropic => "claude-3-5-haiku-20241022",
        LlmProvider::Google => "gemini-3.5-flash",
        LlmProvider::Groq => "llama-3.3-70b-versatile",
        LlmProvider::Together => "meta-llama/Llama-3-8b-chat-hf",
        LlmProvider::Fireworks => "accounts/fireworks/models/llama-v3p1-8b-instruct",
        LlmProvider::DeepSeek => "deepseek-chat",
        LlmProvider::NvidiaNim => "meta/llama3-8b-instruct",
        LlmProvider::Custom => "gpt-4o",
        LlmProvider::None => "mock",
    }
}

fn static_api_key_env(provider: LlmProvider) -> Option<&'static str> {
    match provider {
        LlmProvider::OpenAi => Some("OPENAI_API_KEY"),
        LlmProvider::Grok => Some("XAI_API_KEY"),
        LlmProvider::Anthropic => Some("ANTHROPIC_API_KEY"),
        LlmProvider::Google => Some("GOOGLE_API_KEY"),
        LlmProvider::Groq => Some("GROQ_API_KEY"),
        LlmProvider::Together => Some("TOGETHER_API_KEY"),
        LlmProvider::Fireworks => Some("FIREWORKS_API_KEY"),
        LlmProvider::DeepSeek => Some("DEEPSEEK_API_KEY"),
        LlmProvider::NvidiaNim => Some("NVIDIA_API_KEY"),
        LlmProvider::Custom => Some("RMNG_LLM_API_KEY"),
        LlmProvider::Ollama | LlmProvider::None => None,
    }
}

/// Endpoint from `config/llm-catalog.toml` (or static fallback).
pub fn default_endpoint(provider: LlmProvider) -> Option<String> {
    catalog_endpoint(provider).or_else(|| static_endpoint(provider).map(str::to_string))
}

/// Model id from catalog default entry (or static fallback).
pub fn default_model(provider: LlmProvider) -> String {
    catalog_default_model(provider).unwrap_or_else(|| static_model(provider).to_string())
}

/// API key env var from catalog (or static fallback).
pub fn default_api_key_env(provider: LlmProvider) -> Option<String> {
    catalog_api_key_env(provider).or_else(|| static_api_key_env(provider).map(str::to_string))
}

/// Resolve API key from inline config or environment variable.
pub fn resolve_api_key(cfg: &LlmConfig) -> Result<Option<String>, String> {
    if let Some(key) = &cfg.api_key {
        let trimmed = key.trim();
        if !trimmed.is_empty() {
            return Ok(Some(trimmed.to_string()));
        }
    }
    let env_name = cfg
        .api_key_env_var
        .clone()
        .or_else(|| default_api_key_env(cfg.llm_provider));
    Ok(env_name.and_then(|name| std::env::var(name).ok()))
}

pub fn provider_label(provider: LlmProvider) -> &'static str {
    match provider {
        LlmProvider::None => "none (mock)",
        LlmProvider::Ollama => "ollama",
        LlmProvider::OpenAi => "openai",
        LlmProvider::Grok => "grok",
        LlmProvider::Anthropic => "anthropic",
        LlmProvider::Google => "google",
        LlmProvider::Groq => "groq",
        LlmProvider::Together => "together",
        LlmProvider::Fireworks => "fireworks",
        LlmProvider::DeepSeek => "deepseek",
        LlmProvider::NvidiaNim => "nvidia_nim",
        LlmProvider::Custom => "custom",
    }
}