use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Configured LLM backend identifier (`~/.rmng/config.toml` → `[llm].llm_provider`).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum LlmProviderKind {
    #[serde(rename = "ollama")]
    Ollama,
    #[serde(rename = "openai")]
    OpenAi,
    #[serde(rename = "grok")]
    Grok,
    #[serde(rename = "anthropic")]
    Anthropic,
    #[serde(rename = "google")]
    Google,
    #[serde(rename = "groq")]
    Groq,
    #[serde(rename = "together")]
    Together,
    #[serde(rename = "fireworks")]
    Fireworks,
    #[serde(rename = "deepseek")]
    DeepSeek,
    #[serde(rename = "nvidia_nim")]
    NvidiaNim,
    #[serde(rename = "custom")]
    Custom,
    #[serde(rename = "none")]
    #[default]
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LlmConfig {
    #[serde(default)]
    pub llm_provider: LlmProviderKind,
    pub endpoint_url: Option<String>,
    /// Env var name for API key (preferred over inline `api_key`).
    pub api_key_env_var: Option<String>,
    /// Inline API key — discouraged; use env vars in production.
    #[serde(default)]
    pub api_key: Option<String>,
    pub model: Option<String>,
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,
}

fn default_max_retries() -> u32 {
    2
}

fn default_timeout_secs() -> u64 {
    120
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            llm_provider: LlmProviderKind::None,
            endpoint_url: None,
            api_key_env_var: None,
            api_key: None,
            model: None,
            max_retries: default_max_retries(),
            timeout_secs: default_timeout_secs(),
        }
    }
}

impl LlmConfig {
    pub fn is_mock(&self) -> bool {
        matches!(self.llm_provider, LlmProviderKind::None)
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
        !self.llm.is_mock()
    }
}

/// Backward-compatible alias used across the workspace.
pub type LlmProvider = LlmProviderKind;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ollama_provider() {
        let raw = r#"
[llm]
llm_provider = "ollama"
endpoint_url = "http://127.0.0.1:11434"
model = "llama3.2"
"#;
        let cfg: RmngConfig = toml::from_str(raw).unwrap();
        assert_eq!(cfg.llm.llm_provider, LlmProviderKind::Ollama);
        assert_eq!(
            cfg.llm.endpoint_url.as_deref(),
            Some("http://127.0.0.1:11434")
        );
    }

    #[test]
    fn parses_grok_and_openai() {
        let raw = r#"
[llm]
llm_provider = "grok"
model = "grok-2-latest"
api_key_env_var = "XAI_API_KEY"
max_retries = 3
"#;
        let cfg: RmngConfig = toml::from_str(raw).unwrap();
        assert_eq!(cfg.llm.llm_provider, LlmProviderKind::Grok);
        assert_eq!(cfg.llm.max_retries, 3);

        let raw2 = r#"
[llm]
llm_provider = "openai"
model = "gpt-4o"
"#;
        let cfg2: RmngConfig = toml::from_str(raw2).unwrap();
        assert_eq!(cfg2.llm.llm_provider, LlmProviderKind::OpenAi);
    }

    #[test]
    fn defaults_to_none() {
        let cfg = RmngConfig::default();
        assert_eq!(cfg.llm.llm_provider, LlmProviderKind::None);
        assert!(!cfg.llm_configured());
    }
}