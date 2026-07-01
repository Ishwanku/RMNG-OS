//! Provider abstraction tests (no live API keys required).

use rmng_core::{LlmConfig, LlmProvider};
use rmng_nervous::providers::{
    default_endpoint, default_model, parse_core_intent, resolve_api_key, LlmBackend,
};

#[test]
fn factory_builds_ollama_without_api_key() {
    let cfg = LlmConfig {
        llm_provider: LlmProvider::Ollama,
        endpoint_url: Some("http://127.0.0.1:11434".into()),
        model: Some("llama3.2".into()),
        ..Default::default()
    };
    let backend = LlmBackend::from_config(&cfg).expect("ollama backend");
    assert!(backend.is_some());
    assert_eq!(backend.unwrap().id(), "ollama");
}

#[test]
fn factory_requires_api_key_for_grok() {
    let cfg = LlmConfig {
        llm_provider: LlmProvider::Grok,
        model: Some("grok-2-latest".into()),
        api_key: None,
        api_key_env_var: Some("XAI_API_KEY_TEST_MISSING".into()),
        ..Default::default()
    };
    std::env::remove_var("XAI_API_KEY_TEST_MISSING");
    match LlmBackend::from_config(&cfg) {
        Err(e) => assert!(e.to_string().contains("API key")),
        Ok(_) => panic!("expected missing API key error"),
    }
}

#[test]
fn factory_builds_openai_with_inline_key() {
    let cfg = LlmConfig {
        llm_provider: LlmProvider::OpenAi,
        api_key: Some("sk-test".into()),
        model: Some("gpt-4o".into()),
        ..Default::default()
    };
    let backend = LlmBackend::from_config(&cfg).expect("openai");
    assert_eq!(backend.unwrap().id(), "openai");
}

#[test]
fn defaults_cover_major_providers() {
    assert_eq!(default_endpoint(LlmProvider::Grok).unwrap(), "https://api.x.ai/v1");
    assert_eq!(default_model(LlmProvider::OpenAi), "gpt-4o");
    assert_eq!(default_model(LlmProvider::Grok), "grok-2-latest");
}

#[test]
fn parse_core_intent_strips_markdown_fences() {
    let raw = r#"```json
{"action":"plan.only","reasoning":"done"}
```"#;
    let intent = parse_core_intent(raw).expect("parse");
    assert!(matches!(intent, rmng_core::CoreIntent::PlanOnly { .. }));
}

#[test]
fn resolve_api_key_prefers_env_when_set() {
    std::env::set_var("RMNG_TEST_KEY", "from-env");
    let cfg = LlmConfig {
        llm_provider: LlmProvider::OpenAi,
        api_key_env_var: Some("RMNG_TEST_KEY".into()),
        ..Default::default()
    };
    let key = resolve_api_key(&cfg).expect("resolve");
    assert_eq!(key.as_deref(), Some("from-env"));
    std::env::remove_var("RMNG_TEST_KEY");
}