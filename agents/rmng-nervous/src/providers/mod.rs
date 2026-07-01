//! Pluggable LLM provider adapters for the nervous system (Sprint 5).

mod anthropic;
mod defaults;
mod factory;
mod google;
mod ollama;
mod openai_compat;
mod prompt;
mod types;

pub use anthropic::AnthropicProvider;
pub use defaults::{
    default_api_key_env, default_endpoint, default_model, provider_label, resolve_api_key,
};
pub use factory::{health_check, list_supported_providers, LlmBackend};
pub use google::GoogleProvider;
pub use ollama::OllamaProvider;
pub use openai_compat::OpenAiCompatProvider;
pub use prompt::build_reasoning_prompt;
pub use types::{parse_core_intent, LlmReasonContext, LlmRequest, LlmResponse, ProviderError};