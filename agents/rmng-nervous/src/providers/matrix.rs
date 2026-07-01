use super::defaults::{default_api_key_env, default_endpoint, default_model, resolve_api_key};
use super::factory::LlmBackend;
use super::types::{LlmReasonContext, ProviderError};
use rmng_core::{LlmConfig, LlmProvider};

const PROBE_PROMPT: &str = "Return plan.only JSON: {\"action\":\"plan.only\",\"reasoning\":\"matrix probe ok\"}";

/// One row in the provider validation matrix (Sprint 6).
#[derive(Debug, Clone)]
pub struct MatrixRow {
    pub provider: String,
    pub env_var: Option<String>,
    pub key_set: bool,
    pub health_ok: Option<bool>,
    pub json_ok: Option<bool>,
    pub detail: String,
}

struct MatrixTarget {
    provider: LlmProvider,
    label: &'static str,
}

const TARGETS: &[MatrixTarget] = &[
    MatrixTarget {
        provider: LlmProvider::Grok,
        label: "grok",
    },
    MatrixTarget {
        provider: LlmProvider::OpenAi,
        label: "openai",
    },
    MatrixTarget {
        provider: LlmProvider::Groq,
        label: "groq",
    },
    MatrixTarget {
        provider: LlmProvider::Google,
        label: "google",
    },
    MatrixTarget {
        provider: LlmProvider::Anthropic,
        label: "anthropic",
    },
    MatrixTarget {
        provider: LlmProvider::Together,
        label: "together",
    },
    MatrixTarget {
        provider: LlmProvider::Fireworks,
        label: "fireworks",
    },
    MatrixTarget {
        provider: LlmProvider::DeepSeek,
        label: "deepseek",
    },
    MatrixTarget {
        provider: LlmProvider::NvidiaNim,
        label: "nvidia_nim",
    },
    MatrixTarget {
        provider: LlmProvider::Ollama,
        label: "ollama",
    },
];

/// Run semi-automated provider checks using env vars (no keys in config files).
pub async fn run_provider_matrix() -> Vec<MatrixRow> {
    let mut rows = Vec::new();
    for target in TARGETS {
        rows.push(probe_provider(target.provider, target.label).await);
    }
    rows
}

async fn probe_provider(provider: LlmProvider, label: &str) -> MatrixRow {
    let env_var = default_api_key_env(provider);
    let cfg = LlmConfig {
        llm_provider: provider,
        endpoint_url: default_endpoint(provider),
        model: Some(default_model(provider)),
        api_key_env_var: env_var.clone(),
        ..Default::default()
    };

    let key_set = if provider == LlmProvider::Ollama {
        true
    } else {
        resolve_api_key(&cfg).ok().flatten().is_some()
    };

    if !key_set && provider != LlmProvider::Ollama {
        return MatrixRow {
            provider: label.to_string(),
            env_var,
            key_set: false,
            health_ok: None,
            json_ok: None,
            detail: "skipped — API key not set in environment".into(),
        };
    }

    let backend = match LlmBackend::from_config(&cfg) {
        Ok(Some(b)) => b,
        Ok(None) => {
            return MatrixRow {
                provider: label.to_string(),
                env_var,
                key_set,
                health_ok: None,
                json_ok: None,
                detail: "skipped — mock provider".into(),
            };
        }
        Err(e) => {
            return MatrixRow {
                provider: label.to_string(),
                env_var,
                key_set,
                health_ok: Some(false),
                json_ok: None,
                detail: format!("config error: {e}"),
            };
        }
    };

    let health_ok = backend.health().await.ok();
    let ctx = LlmReasonContext::default();
    let json_ok = match backend.reason_core(PROBE_PROMPT, &ctx).await {
        Ok(intent) => {
            let ok = matches!(intent, rmng_core::CoreIntent::PlanOnly { .. });
            (Some(ok), "core-intent parse ok".to_string())
        }
        Err(ref e @ ProviderError::Api { status, ref message, .. }) => (
            Some(false),
            format!("API {status} [{:?}]: {message}", e.kind()),
        ),
        Err(e) => (
            Some(false),
            format!("reason failed [{:?}]: {e}", e.kind()),
        ),
    };

    let detail = if health_ok == Some(false) {
        "health probe failed".into()
    } else if json_ok.0 == Some(false) {
        json_ok.1
    } else if json_ok.0 == Some(true) {
        "health + JSON intent ok".into()
    } else {
        json_ok.1
    };

    MatrixRow {
        provider: label.to_string(),
        env_var,
        key_set,
        health_ok,
        json_ok: json_ok.0,
        detail,
    }
}