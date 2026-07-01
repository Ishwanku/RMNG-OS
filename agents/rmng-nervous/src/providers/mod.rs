//! Pluggable LLM provider adapters for the nervous system (Sprint 5).

mod anthropic;
mod catalog;
mod defaults;
mod factory;
mod google;
mod matrix;
mod ollama;
mod openai_compat;
mod prompt;
mod reason;
mod types;

pub use anthropic::AnthropicProvider;
pub use catalog::{
    catalog_path, install_user_catalog, list_all_providers, list_catalog_models, load_catalog,
    ModelEntry, ProviderEntry,
};
pub use defaults::{
    default_api_key_env, default_endpoint, default_model, provider_label, resolve_api_key,
};
pub use factory::{health_check, health_check_detailed, list_supported_providers, HealthReport, LlmBackend};
pub use matrix::{run_provider_matrix, MatrixRow};
pub use google::GoogleProvider;
pub use ollama::OllamaProvider;
pub use openai_compat::OpenAiCompatProvider;
pub use prompt::build_reasoning_prompt;
pub use types::{parse_core_intent, LlmReasonContext, LlmRequest, LlmResponse, ProviderError};