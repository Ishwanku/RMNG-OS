use crate::mock::mock_core_intent;
use crate::ollama::OllamaAdapter;
use crate::skill::{assemble_prompt, AgentSkill};
use rmng_core::{CoreIntent, LlmProvider, RmngConfig};

#[derive(Debug, thiserror::Error)]
pub enum ConnectorError {
    #[error("provider not implemented: {0}")]
    NotImplemented(String),
    #[error("provider misconfigured: {0}")]
    Misconfigured(String),
    #[error("nervous adapter error: {0}")]
    Adapter(#[from] crate::ollama::NervousError),
    #[error("runtime error: {0}")]
    Runtime(#[from] rmng_core::RmngError),
}

pub struct NervousConnector {
    config: RmngConfig,
}

impl NervousConnector {
    pub fn from_config(config: RmngConfig) -> Self {
        Self { config }
    }

    pub fn load() -> Self {
        Self::from_config(RmngConfig::load())
    }

    /// Resolve user prompt (+ optional skill) to a v2 `CoreIntent`. Never executes tools.
    pub async fn reason_core(
        &self,
        prompt: &str,
        skill_name: Option<&str>,
        skill: Option<&AgentSkill>,
    ) -> Result<CoreIntent, ConnectorError> {
        let _assembled = assemble_prompt(skill, prompt);

        match self.config.llm.llm_provider {
            LlmProvider::None => Ok(mock_core_intent(
                prompt,
                skill_name,
                skill.map(|s| s.instructions.as_str()),
            )),
            LlmProvider::Ollama => {
                let url = self
                    .config
                    .llm
                    .endpoint_url
                    .as_deref()
                    .unwrap_or("http://127.0.0.1:11434");
                let model = self.config.llm.model.as_deref().unwrap_or("llama3.2");
                let adapter = OllamaAdapter::new(url, model);
                Ok(adapter.reason_core(&_assembled, skill_name).await?)
            }
            LlmProvider::OpenAi | LlmProvider::Anthropic | LlmProvider::Custom => {
                Err(ConnectorError::NotImplemented(format!(
                    "{:?} connector not yet wired — execution plane only",
                    self.config.llm.llm_provider
                )))
            }
        }
    }

    pub fn provider_label(&self) -> &'static str {
        match self.config.llm.llm_provider {
            LlmProvider::None => "none (mock)",
            LlmProvider::Ollama => "ollama",
            LlmProvider::OpenAi => "openai",
            LlmProvider::Anthropic => "anthropic",
            LlmProvider::Custom => "custom",
        }
    }
}