//! Shared helpers for live LLM orchestration integration tests (Sprint 31).

use rmng_core::{LlmConfig, LlmProvider, RmngConfig, SessionStore, ToolResultRecord};
use rmng_nervous::{AgentRouter, NervousConnector, RouteOutcome};
use chrono::Utc;

pub fn chain_strict() -> bool {
    std::env::var("RMNG_CHAIN_STRICT")
        .map(|v| v != "0" && v != "false")
        .unwrap_or(true)
}

pub fn groq_config() -> Option<RmngConfig> {
    if std::env::var("GROQ_API_KEY").is_err() {
        return None;
    }
    Some(RmngConfig {
        llm: LlmConfig {
            llm_provider: LlmProvider::Groq,
            model: Some("llama-3.3-70b-versatile".into()),
            ..Default::default()
        },
        profile: Some("groq-fast".into()),
        profiles: vec![],
        ..Default::default()
    })
}

pub fn grok_config() -> Option<RmngConfig> {
    if std::env::var("XAI_API_KEY").is_err() {
        return None;
    }
    Some(RmngConfig {
        llm: LlmConfig {
            llm_provider: LlmProvider::Grok,
            model: Some("grok-3-mini".into()),
            ..Default::default()
        },
        profile: Some("grok-frontier".into()),
        profiles: vec![],
        ..Default::default()
    })
}

pub fn openai_config() -> Option<RmngConfig> {
    if std::env::var("OPENAI_API_KEY").is_err() {
        return None;
    }
    Some(RmngConfig {
        llm: LlmConfig {
            llm_provider: LlmProvider::OpenAi,
            model: Some("gpt-4o-mini".into()),
            ..Default::default()
        },
        ..Default::default()
    })
}

pub fn anthropic_config() -> Option<RmngConfig> {
    if std::env::var("ANTHROPIC_API_KEY").is_err() {
        return None;
    }
    Some(RmngConfig {
        llm: LlmConfig {
            llm_provider: LlmProvider::Anthropic,
            model: Some("claude-3-5-haiku-20241022".into()),
            ..Default::default()
        },
        ..Default::default()
    })
}

pub fn google_config() -> Option<RmngConfig> {
    if std::env::var("GOOGLE_API_KEY").is_err() {
        return None;
    }
    Some(RmngConfig {
        llm: LlmConfig {
            llm_provider: LlmProvider::Google,
            model: Some("gemini-2.0-flash".into()),
            ..Default::default()
        },
        ..Default::default()
    })
}

pub async fn ollama_config() -> Option<RmngConfig> {
    let ok = rmng_nervous::OllamaProvider::default()
        .health()
        .await
        .unwrap_or(false);
    if !ok {
        return None;
    }
    Some(RmngConfig {
        llm: LlmConfig {
            llm_provider: LlmProvider::Ollama,
            model: Some("llama3.2".into()),
            endpoint_url: Some("http://127.0.0.1:11434".into()),
            ..Default::default()
        },
        ..Default::default()
    })
}

pub fn git_chain_prompt(session_id: &str) -> String {
    format!(
        r#"User request: Coordinate a git hygiene workflow across multiple agents.

You are swarm-coordinator (L4). Emit exactly ONE plan.only JSON object.

REQUIRED:
- metadata.session_id = "{session_id}"
- metadata.chain_id = "{session_id}"
- metadata.handoff_chain = ["swarm-coordinator","repo-keeper","runtime-executor"] as a JSON array (NOT a comma string)

Do NOT use markdown fences. Do NOT use handoff_to when a chain is needed."#
    )
}

pub fn return_to_prompt(session_id: &str) -> String {
    format!(
        r#"You are repo-keeper (L3 specialist). recent_tool_results shows git.status succeeded with a clean working tree.

Emit exactly ONE plan.only JSON object returning control to the orchestrator.

REQUIRED:
- metadata.session_id = "{session_id}"
- metadata.handoff_return_to = "swarm-coordinator"
- reasoning must summarize the git status result

Do NOT emit handoff_chain, handoff_to, or tool.execute. No markdown fences."#
    )
}

pub fn assert_handoff_chain(outcome: &RouteOutcome, label: &str) {
    match outcome {
        RouteOutcome::HandoffChain { chain, .. } => {
            assert!(chain.len() >= 2, "[{label}] chain too short: {chain:?}");
            assert_eq!(chain[0], "swarm-coordinator", "[{label}] bad chain start");
            eprintln!("[{label}] OK HandoffChain: {chain:?}");
        }
        RouteOutcome::Handoff { to_agent, .. } => {
            eprintln!("[{label}] WARN Handoff to {to_agent}");
            if chain_strict() {
                panic!("[{label}] expected HandoffChain; set RMNG_CHAIN_STRICT=0 to allow fallback");
            }
        }
        other => panic!("[{label}] unexpected: {other:?}"),
    }
}

pub fn assert_return_to_orchestrator(outcome: &RouteOutcome, label: &str) {
    match outcome {
        RouteOutcome::Handoff { to_agent, .. } => {
            assert_eq!(
                to_agent, "swarm-coordinator",
                "[{label}] expected return to swarm-coordinator, got {to_agent}"
            );
            eprintln!("[{label}] OK handoff_return_to -> {to_agent}");
        }
        other => panic!("[{label}] expected Handoff return, got {other:?}"),
    }
}

pub async fn router_with_config(
    cfg: RmngConfig,
    label: &str,
) -> (AgentRouter, SessionStore, String, std::path::PathBuf) {
    let dir = std::env::temp_dir().join(format!("rmng-live-{label}-{}", uuid::Uuid::new_v4()));
    let store = SessionStore::new(&dir);
    let session = store.create().expect("create");
    let session_id = session.id.clone();
    let store_clone = store.clone();
    let registry = rmng_nervous::AgentRegistry::load().expect("registry");
    let router = AgentRouter::with_session_store(registry, NervousConnector::from_config(cfg), store);
    (router, store_clone, session_id, dir)
}

pub fn seed_repo_keeper_tool_result(store: &SessionStore, session_id: &str) {
    let mut session = store.load(session_id).expect("load");
    store
        .record_tool_result(
            &mut session,
            ToolResultRecord {
                timestamp: Utc::now(),
                tool: "git.status".into(),
                parameters: serde_json::json!({}),
                output: "clean working tree".into(),
                success: true,
                exit_code: Some(0),
                handoff_from: Some("repo-keeper".into()),
                peak_rss_kb: None,
                cpu_time_ms: None,
                runtime_ms: None,
            },
        )
        .expect("tool result");
}