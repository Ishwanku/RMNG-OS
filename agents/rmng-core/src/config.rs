use rmng_mcp::IsolationLimits;
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
    /// Generation overrides (Sprint 7) — provider defaults when unset.
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub top_p: Option<f32>,
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
            temperature: None,
            max_tokens: None,
            top_p: None,
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
    let raw = format!("[llm]
llm_provider = \"{}\"", s);
    toml::from_str::<RmngConfig>(&raw)
        .map(|c| c.llm.llm_provider)
        .map_err(|e| format!("unknown provider '{}': {}", s, e))
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
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub top_p: Option<f32>,
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
        if let Some(v) = self.temperature {
            base.temperature = Some(v);
        }
        if let Some(v) = self.max_tokens {
            base.max_tokens = Some(v);
        }
        if let Some(v) = self.top_p {
            base.top_p = Some(v);
        }
    }
}

/// Per-agent LLM override surface (from `agents/definitions/*.yaml`).
pub trait AgentLlmOverride {
    fn llm_profile_name(&self) -> Option<&str>;
    fn llm_provider_override(&self) -> Option<LlmProviderKind>;
    fn model_override(&self) -> Option<&str>;
    /// Profile names to try after primary fails (Sprint 8). Empty → use global `llm_fallback`.
    fn llm_fallback_profiles(&self) -> &[String] {
        &[]
    }
}

/// One step in a resolved LLM fallback chain.
#[derive(Debug, Clone)]
pub struct LlmConfigEntry {
    pub label: String,
    pub config: LlmConfig,
}

/// LLM spend governance (Sprint 11) — opt-in warn/deny thresholds.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum BudgetEnforceMode {
    #[default]
    Off,
    Warn,
    Deny,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LlmBudgetConfig {
    /// Daily USD cap (global). Omit to disable budget checks.
    #[serde(default)]
    pub daily_usd: Option<f64>,
    /// Fraction of daily_usd to emit warn (default 0.8).
    #[serde(default)]
    pub warn_threshold: Option<f64>,
    /// Fraction of daily_usd to block new calls when enforce=deny (default 1.0).
    #[serde(default)]
    pub deny_threshold: Option<f64>,
    #[serde(default)]
    pub enforce: BudgetEnforceMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RmngConfig {
    #[serde(default)]
    pub llm: LlmConfig,
    /// Active profile name from `[[llm.profiles]]`.
    pub profile: Option<String>,
    #[serde(default)]
    pub profiles: Vec<LlmProfile>,
    /// Global fallback profile names (Sprint 8) — tried in order when primary LLM fails.
    #[serde(default)]
    pub llm_fallback: Vec<String>,
    /// Default subprocess isolation for MCP tools (Sprint 10).
    #[serde(default)]
    pub isolation: IsolationLimits,
    /// LLM budget caps and enforcement (Sprint 11).
    #[serde(default)]
    pub llm_budget: LlmBudgetConfig,
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

    /// Global config merged with per-agent overrides (profile → provider → model).
    pub fn resolved_llm_for_agent(&self, agent: &dyn AgentLlmOverride) -> LlmConfig {
        let mut llm = if let Some(name) = agent.llm_profile_name() {
            let mut base = self.llm.clone();
            if let Some(p) = self.profiles.iter().find(|p| p.name == name) {
                p.apply_to(&mut base);
                base
            } else {
                tracing::warn!(profile = name, "agent llm_profile not found; using global");
                self.resolved_llm()
            }
        } else {
            self.resolved_llm()
        };
        if let Some(p) = agent.llm_provider_override() {
            llm.llm_provider = p;
        }
        if let Some(m) = agent.model_override() {
            llm.model = Some(m.to_string());
        }
        llm
    }

    /// Resolve a named `[[profiles]]` entry to a full `LlmConfig`.
    pub fn resolve_profile_by_name(&self, name: &str) -> Option<LlmConfig> {
        let mut base = self.llm.clone();
        let p = self.profiles.iter().find(|p| p.name == name)?;
        p.apply_to(&mut base);
        Some(base)
    }

    /// Primary LLM config plus ordered fallbacks (agent overrides global list).
    pub fn resolved_llm_chain_for_agent(&self, agent: Option<&dyn AgentLlmOverride>) -> Vec<LlmConfigEntry> {
        let primary = match agent {
            Some(a) => self.resolved_llm_for_agent(a),
            None => self.resolved_llm(),
        };
        let mut chain = vec![LlmConfigEntry {
            label: "primary".into(),
            config: primary,
        }];

        let fallback_names: Vec<String> = agent
            .map(|a| a.llm_fallback_profiles().to_vec())
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| self.llm_fallback.clone());

        for name in fallback_names {
            let Some(cfg) = self.resolve_profile_by_name(&name) else {
                tracing::warn!(profile = %name, "llm_fallback profile not found; skipping");
                continue;
            };
            let duplicate = chain.last().is_some_and(|e| {
                e.config.llm_provider == cfg.llm_provider && e.config.model == cfg.model
            });
            if !duplicate {
                chain.push(LlmConfigEntry {
                    label: format!("fallback:{name}"),
                    config: cfg,
                });
            }
        }
        chain
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

    struct TestAgent {
        profile: Option<String>,
        provider: Option<LlmProviderKind>,
        model: Option<String>,
        fallback: Vec<String>,
    }

    impl AgentLlmOverride for TestAgent {
        fn llm_profile_name(&self) -> Option<&str> {
            self.profile.as_deref()
        }
        fn llm_provider_override(&self) -> Option<LlmProviderKind> {
            self.provider
        }
        fn model_override(&self) -> Option<&str> {
            self.model.as_deref()
        }
        fn llm_fallback_profiles(&self) -> &[String] {
            &self.fallback
        }
    }

    #[test]
    fn resolves_per_agent_llm_overrides() {
        let raw = r#"
[llm]
llm_provider = "none"

[[profiles]]
name = "groq-fast"
llm_provider = "groq"
model = "llama-3.3-70b-versatile"
api_key_env_var = "GROQ_API_KEY"
"#;
        let cfg: RmngConfig = toml::from_str(raw).unwrap();
        let agent = TestAgent {
            profile: Some("groq-fast".into()),
            provider: None,
            model: None,
            fallback: vec![],
        };
        let llm = cfg.resolved_llm_for_agent(&agent);
        assert_eq!(llm.llm_provider, LlmProviderKind::Groq);

        let agent2 = TestAgent {
            profile: None,
            provider: Some(LlmProviderKind::Grok),
            model: Some("grok-4.3".to_string()),
            fallback: vec![],
        };
        let llm2 = cfg.resolved_llm_for_agent(&agent2);
        assert_eq!(llm2.llm_provider, LlmProviderKind::Grok);
        assert_eq!(llm2.model.as_deref(), Some("grok-4.3"));
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

    #[test]
    fn resolves_llm_fallback_chain_global_and_per_agent() {
        let raw = r#"
llm_fallback = ["grok-frontier", "local-ollama"]

[llm]
llm_provider = "groq"
model = "llama-3.3-70b-versatile"
api_key_env_var = "GROQ_API_KEY"

[[profiles]]
name = "grok-frontier"
llm_provider = "grok"
model = "grok-4.3"
api_key_env_var = "XAI_API_KEY"

[[profiles]]
name = "local-ollama"
llm_provider = "ollama"
endpoint_url = "http://127.0.0.1:11434"
model = "llama3.2"
"#;
        let cfg: RmngConfig = toml::from_str(raw).unwrap();
        let global = cfg.resolved_llm_chain_for_agent(None);
        assert_eq!(global.len(), 3);
        assert_eq!(global[0].label, "primary");
        assert_eq!(global[0].config.llm_provider, LlmProviderKind::Groq);
        assert_eq!(global[1].label, "fallback:grok-frontier");
        assert_eq!(global[2].label, "fallback:local-ollama");

        let agent = TestAgent {
            profile: None,
            provider: None,
            model: None,
            fallback: vec!["local-ollama".into()],
        };
        let per_agent = cfg.resolved_llm_chain_for_agent(Some(&agent));
        assert_eq!(per_agent.len(), 2);
        assert_eq!(per_agent[1].label, "fallback:local-ollama");
    }
}