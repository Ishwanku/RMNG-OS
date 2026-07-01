pub mod agent;
pub mod connector;
pub mod layer;
pub mod mock;
pub mod nervous_audit;
pub mod providers;
pub mod router;
pub mod skill;

pub use agent::{AgentDefinition, AgentError, AgentRegistry};
pub use connector::{ConnectorError, NervousConnector};
pub use layer::{AgentLayer, LayerAgent};
pub use providers::{
    catalog_path, default_endpoint, default_model, health_check, health_check_detailed,
    install_user_catalog, list_all_providers, list_catalog_models, list_supported_providers,
    compare_models, fetch_live_models, load_catalog, parse_core_intent, resolve_api_key,
    run_provider_matrix, HealthReport, LlmBackend, LlmReasonContext, MatrixRow, ModelEntry,
    ModelSyncReport, OllamaProvider, ProviderEntry, ProviderError, ProviderErrorKind,
};
pub use router::{AgentRoute, AgentRouter, RouteOutcome, RouterError};
pub use skill::{load_skill, load_skill_index, load_skills_for_agent, AgentSkill, SkillError, SkillSummary};

/// Backward-compatible alias (Sprint 4c -> Sprint 5).
pub type OllamaAdapter = OllamaProvider;
