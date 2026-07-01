//! Pluggable LLM provider adapters for the nervous system (Sprint 5).

mod anthropic;
mod backoff;
mod catalog;
mod circuit_breaker;
mod cost;
mod defaults;
mod discovery;
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
    apply_live_models, catalog_model_pricing, catalog_path, install_user_catalog,
    list_all_providers, list_catalog_models, load_catalog, provider_id, user_catalog_path,
    ModelEntry, ProviderEntry,
};
pub use cost::resolve_model_pricing;
pub use discovery::{compare_models, fetch_live_models, ModelSyncReport};
pub use defaults::{
    default_api_key_env, default_endpoint, default_model, provider_label, resolve_api_key,
};
pub use factory::{health_check, health_check_detailed, list_supported_providers, HealthReport, LlmBackend};
pub use matrix::{run_provider_matrix, MatrixRow};
pub use google::GoogleProvider;
pub use ollama::OllamaProvider;
pub use openai_compat::OpenAiCompatProvider;
pub use prompt::build_reasoning_prompt;
pub use circuit_breaker::{
    allow_request, circuit_state_path, list_circuit_statuses, record_failure, record_success,
    reload_from_disk, CircuitStatus,
};
pub use types::{
    parse_core_intent, LlmReasonContext, LlmRequest, LlmResponse, LlmUsage, ProviderError,
    ProviderErrorKind, ReasonResult,
};