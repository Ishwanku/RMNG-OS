use crate::ollama::OllamaAdapter;
use crate::mock::{mock_intent, mock_intent_for_tool};
use rmng_core::{Intent, LlmProvider, RmngConfig, RmngError};

#[derive(Debug, thiserror::Error)]
pub enum ConnectorError {
    #[error("provider not implemented: {0}")]
    NotImplemented(String),
    #[error("provider misconfigured: {0}")]
    Misconfigured(String),
    #[error("nervous adapter error: {0}")]
    Adapter(#[from] crate::ollama::NervousError),
    #[error("runtime error: {0}")]
    Runtime(#[from] RmngError),
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

    /// Resolve user prompt to a JSON intent. Never executes tools.
    pub async fn reason(&self, prompt: &str) -> Result<Intent, ConnectorError> {
        match self.config.llm.llm_provider {
            LlmProvider::None => {
                // Keyword hint for local testing without any inference engine
                let lower = prompt.to_lowercase();
                if lower.contains("git") {
                    return mock_intent_for_tool(prompt, "git.status")
                        .map_err(ConnectorError::Runtime);
                }
                if lower.contains("kernel") || lower.contains("build") {
                    return mock_intent_for_tool(prompt, "kernel.status")
                        .map_err(ConnectorError::Runtime);
                }
                Ok(mock_intent(prompt))
            }
            LlmProvider::Ollama => {
                let url = self
                    .config
                    .llm
                    .endpoint_url
                    .as_deref()
                    .unwrap_or("http://127.0.0.1:11434");
                let model = self.config.llm.model.as_deref().unwrap_or("llama3.2");
                let adapter = OllamaAdapter::new(url, model);
                Ok(adapter.reason(prompt).await?)
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
