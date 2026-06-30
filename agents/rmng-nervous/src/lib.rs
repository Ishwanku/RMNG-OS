pub mod connector;
pub mod mock;
pub mod ollama;
pub mod skill;

pub use connector::{ConnectorError, NervousConnector};
pub use ollama::OllamaAdapter;
pub use skill::{load_skill, AgentSkill, SkillError};