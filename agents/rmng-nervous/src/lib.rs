pub mod agent;
pub mod connector;
pub mod layer;
pub mod mock;
pub mod ollama;
pub mod router;
pub mod skill;

pub use agent::{AgentDefinition, AgentError, AgentRegistry};
pub use connector::{ConnectorError, NervousConnector};
pub use layer::{AgentLayer, LayerAgent};
pub use ollama::{LlmReasonContext, OllamaAdapter};
pub use router::{AgentRoute, AgentRouter, RouteOutcome, RouterError};
pub use skill::{load_skill, load_skill_index, load_skills_for_agent, AgentSkill, SkillError, SkillSummary};
