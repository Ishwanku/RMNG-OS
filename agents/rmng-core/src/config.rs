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

/// Parse provider id from CLI/config strings (`grok`, `google`, …).
pub fn parse_provider_str(s: &str) -> Result<LlmProviderKind, String> {
    let wrapped = format!(r#"llm_provider = "{s}""#);
    #[derive(Deserialize)]
    struct Wrap {
        llm_provider: LlmProviderKind,
    }
    let raw = format!("[llm]\n{wrapped}");
    toml::from_str::<RmngConfig>(&raw)
        .map(|c| c.llm.llm_provider)
        .map_err(|e| format!("unknown provider '{s}': {e}"))
}

/// Named LLM preset — switch with `[llm] profile = "name"` or `rmng llm use <name>`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LlmProfile {
    pub name: String,
    #[serde(default)]
    pub llm_provider: Option<LlmProviderKind>,
    pub endpoint_url: Option<String>,
    pub api_key_env_var: Option<String>,
    pub model: Option<String>,
    pub max_retries: Option<u32>,
    pub timeout_secs: Option<u64>,
}

impl LlmProfile {
    pub fn apply_to(&self, base: &mut LlmConfig) {
        if let Some(p) = self.llm_provider {
            base.llm_provider = p;
        }
        if let Some(v) = &self.endpoint_url {
            base.endpoint_url = Some(v.clone());
        }
        if let Some(v) = &self.api_key_env_var {
            base.api_key_env_var = Some(v.clone());
        }
        if let Some(v) = &self.model {
            base.model = Some(v.clone());
        }
        if let Some(v) = self.max_retries {
            base.max_retries = v;
        }
        if let Some(v) = self.timeout_secs {
            base.timeout_secs = v;
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RmngConfig {
    #[serde(default)]
    pub llm: LlmConfig,
    /// Active profile name from `[[llm.profiles]]`.
    pub profile: Option<String>,
    #[serde(default)]
    pub profiles: Vec<LlmProfile>,
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
        !self.resolved_llm().is_mock()
    }

    /// Merge `[llm]` with active `profile` (if set).
    pub fn resolved_llm(&self) -> LlmConfig {
        let mut out = self.llm.clone();
        if let Some(name) = &self.profile {
            if let Some(p) = self.profiles.iter().find(|p| p.name == *name) {
                p.apply_to(&mut out);
            }
        }
        out
    }

    /// Apply one-off overrides (CLI flags) on top of resolved config.
    pub fn with_llm_overrides(
        &self,
        provider: Option<LlmProviderKind>,
        model: Option<String>,
        profile: Option<String>,
    ) -> RmngConfig {
        let mut cfg = self.clone();
        if let Some(name) = profile {
            cfg.profile = Some(name);
        }
        let mut llm = cfg.resolved_llm();
        if let Some(p) = provider {
            llm.llm_provider = p;
        }
        if let Some(m) = model {
            llm.model = Some(m);
        }
        cfg.llm = llm;
        cfg.profile = None;
        cfg
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

    #[test]
    fn resolves_named_profile() {
        let raw = r#"
profile = "gemini-fast"

[llm]
llm_provider = "none"

[[profiles]]
name = "gemini-fast"
llm_provider = "google"
model = "gemini-3.5-flash"
api_key_env_var = "GOOGLE_API_KEY"
"#;
        let cfg: RmngConfig = toml::from_str(raw).unwrap();
        let llm = cfg.resolved_llm();
        assert_eq!(llm.llm_provider, LlmProviderKind::Google);
        assert_eq!(llm.model.as_deref(), Some("gemini-3.5-flash"));
    }
}