//! Live provider matrix — requires API keys in environment.
//! Run: `cargo test -p rmng-nervous provider_matrix -- --ignored --nocapture`

use rmng_core::{LlmConfig, LlmProvider};
use rmng_nervous::providers::{default_model, LlmBackend, resolve_api_key};

fn has_key(provider: LlmProvider, env: &str) -> bool {
    let cfg = LlmConfig {
        llm_provider: provider,
        api_key_env_var: Some(env.into()),
        ..Default::default()
    };
    resolve_api_key(&cfg).ok().flatten().is_some()
}

#[tokio::test]
#[ignore = "requires XAI_API_KEY"]
async fn matrix_grok_live() {
    if !has_key(LlmProvider::Grok, "XAI_API_KEY") {
        eprintln!("skip: XAI_API_KEY not set");
        return;
    }
    let cfg = LlmConfig {
        llm_provider: LlmProvider::Grok,
        model: Some(default_model(LlmProvider::Grok).into()),
        api_key_env_var: Some("XAI_API_KEY".into()),
        ..Default::default()
    };
    let backend = LlmBackend::from_config(&cfg).expect("grok backend");
    let b = backend.expect("some backend");
    assert!(b.health().await.expect("health"));
}

#[tokio::test]
#[ignore = "requires OPENAI_API_KEY"]
async fn matrix_openai_live() {
    if !has_key(LlmProvider::OpenAi, "OPENAI_API_KEY") {
        eprintln!("skip: OPENAI_API_KEY not set");
        return;
    }
    let cfg = LlmConfig {
        llm_provider: LlmProvider::OpenAi,
        model: Some("gpt-4o-mini".into()),
        api_key_env_var: Some("OPENAI_API_KEY".into()),
        ..Default::default()
    };
    let backend = LlmBackend::from_config(&cfg).expect("openai backend");
    let b = backend.expect("some backend");
    assert!(b.health().await.expect("health"));
}

#[tokio::test]
#[ignore = "requires GROQ_API_KEY"]
async fn matrix_groq_live() {
    if !has_key(LlmProvider::Groq, "GROQ_API_KEY") {
        eprintln!("skip: GROQ_API_KEY not set");
        return;
    }
    let cfg = LlmConfig {
        llm_provider: LlmProvider::Groq,
        api_key_env_var: Some("GROQ_API_KEY".into()),
        ..Default::default()
    };
    let backend = LlmBackend::from_config(&cfg).expect("groq backend");
    assert!(backend.expect("backend").health().await.expect("health"));
}

#[tokio::test]
#[ignore = "requires GOOGLE_API_KEY"]
async fn matrix_google_live() {
    if !has_key(LlmProvider::Google, "GOOGLE_API_KEY") {
        eprintln!("skip: GOOGLE_API_KEY not set");
        return;
    }
    let cfg = LlmConfig {
        llm_provider: LlmProvider::Google,
        api_key_env_var: Some("GOOGLE_API_KEY".into()),
        ..Default::default()
    };
    let backend = LlmBackend::from_config(&cfg).expect("google backend");
    assert!(backend.expect("backend").health().await.expect("health"));
}

#[tokio::test]
#[ignore = "requires local Ollama"]
async fn matrix_ollama_live() {
    let cfg = LlmConfig {
        llm_provider: LlmProvider::Ollama,
        endpoint_url: Some("http://127.0.0.1:11434".into()),
        model: Some("llama3.2".into()),
        ..Default::default()
    };
    let backend = LlmBackend::from_config(&cfg).expect("ollama backend");
    let b = backend.expect("backend");
    if !b.health().await.unwrap_or(false) {
        eprintln!("skip: ollama not reachable");
        return;
    }
}

#[test]
fn matrix_module_runs_without_keys() {
    // Smoke: run_provider_matrix should not panic when keys are absent.
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    let rows = rt.block_on(rmng_nervous::run_provider_matrix());
    assert!(!rows.is_empty());
    assert!(rows.iter().any(|r| r.provider == "grok"));
}