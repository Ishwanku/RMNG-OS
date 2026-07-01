pub mod agent;
pub mod chain;
pub mod connector;
pub mod layer;
pub mod orchestration_prompt;
mod mock;
pub mod nervous_audit;
pub mod providers;
pub mod router;
pub mod skill;

pub use agent::{AgentDefinition, AgentError, AgentRegistry};
pub use connector::{ConnectorError, NervousConnector};
pub use layer::{AgentLayer, LayerAgent};
pub use providers::{
    allow_request, apply_live_models, catalog_model_pricing, catalog_path, circuit_state_path,
    compare_models, default_endpoint, default_model, fetch_live_models, health_check,
    health_check_detailed, install_user_catalog, list_all_providers, list_catalog_models,
    list_circuit_statuses, list_supported_providers, load_catalog, parse_core_intent,
    record_failure, record_success, reload_from_disk, resolve_api_key, run_provider_matrix,
    provider_id, resolve_model_pricing, user_catalog_path, CircuitStatus, HealthReport, LlmBackend,
    LlmReasonContext, LlmUsage,
    MatrixRow, ModelEntry, ModelSyncReport, OllamaProvider, ProviderEntry, ProviderError,
    ProviderErrorKind, ReasonResult,
};
pub use router::{AgentRoute, AgentRouter, RouteOutcome, RouterError};
pub use skill::{load_skill, load_skill_index, load_skills_for_agent, AgentSkill, SkillError, SkillSummary};

/// Backward-compatible alias (Sprint 4c -> Sprint 5).
pub type OllamaAdapter = OllamaProvider;
