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
    default_endpoint, default_model, health_check, health_check_detailed, list_supported_providers,
    parse_core_intent, resolve_api_key, run_provider_matrix, HealthReport, LlmBackend,
    LlmReasonContext, MatrixRow, OllamaProvider, ProviderError,
};
pub use router::{AgentRoute, AgentRouter, RouteOutcome, RouterError};
pub use skill::{load_skill, load_skill_index, load_skills_for_agent, AgentSkill, SkillError, SkillSummary};

/// Backward-compatible alias (Sprint 4c -> Sprint 5).
pub type OllamaAdapter = OllamaProvider;
