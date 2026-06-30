use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum LlmProvider {
    Ollama,
    OpenAi,
    Anthropic,
    Custom,
    #[default]
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LlmConfig {
    #[serde(default)]
    pub llm_provider: LlmProvider,
    pub endpoint_url: Option<String>,
    pub api_key_env_var: Option<String>,
    pub model: Option<String>,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            llm_provider: LlmProvider::None,
            endpoint_url: None,
            api_key_env_var: None,
            model: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RmngConfig {
    #[serde(default)]
    pub llm: LlmConfig,
}

impl RmngConfig {
    pub fn config_path() -> PathBuf {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join(".rmng/config.toml");
        }
        PathBuf::from("/tmp/rmng/config.toml")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if !path.exists() {
            return Self::default();
        }
        let raw = std::fs::read_to_string(&path).unwrap_or_default();
        toml::from_str(&raw).unwrap_or_else(|e| {
            tracing::warn!(error = %e, path = %path.display(), "invalid config; using defaults");
            Self::default()
        })
    }

    pub fn llm_configured(&self) -> bool {
        !matches!(self.llm.llm_provider, LlmProvider::None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_provider_block() {
        let raw = r#"
[llm]
llm_provider = "ollama"
endpoint_url = "http://127.0.0.1:11434"
model = "llama3.2"
"#;
        let cfg: RmngConfig = toml::from_str(raw).unwrap();
        assert_eq!(cfg.llm.llm_provider, LlmProvider::Ollama);
        assert_eq!(
            cfg.llm.endpoint_url.as_deref(),
            Some("http://127.0.0.1:11434")
        );
    }

    #[test]
    fn defaults_to_none() {
        let cfg = RmngConfig::default();
        assert_eq!(cfg.llm.llm_provider, LlmProvider::None);
        assert!(!cfg.llm_configured());
    }
}
