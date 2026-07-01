pub mod agent;
pub mod connector;
pub mod mock;
pub mod ollama;
pub mod router;
pub mod skill;

pub use agent::{AgentDefinition, AgentError, AgentRegistry};
pub use connector::{ConnectorError, NervousConnector};
pub use ollama::OllamaAdapter;
pub use router::{AgentRoute, AgentRouter, RouterError};
pub use skill::{load_skill, load_skill_index, load_skills_for_agent, AgentSkill, SkillError, SkillSummary};