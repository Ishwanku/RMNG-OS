//! Sprint 25 — auto-continue foundation (session-backed continuation state).

use rmng_core::{ContinuationStatus, LlmConfig, LlmProvider, RmngConfig, SessionStore};
use rmng_nervous::{AutoContinueLoop, AutoContinueStep, AgentRouter};

fn mock_connector() -> rmng_nervous::NervousConnector {
    rmng_nervous::NervousConnector::from_config(RmngConfig {
        llm: LlmConfig {
            llm_provider: LlmProvider::None,
            ..Default::default()
        },
        profile: None,
        profiles: vec![],
        ..Default::default()
    })
}

fn test_router(store: SessionStore) -> AgentRouter {
    let registry = rmng_nervous::AgentRegistry::load().expect("registry");
    AgentRouter::with_session_store(registry, mock_connector(), store)
}

#[tokio::test]
async fn continuation_state_persisted_on_begin() {
    let dir = std::env::temp_dir().join(format!("rmng-cont-{}", uuid::Uuid::new_v4()));
    let store = SessionStore::new(&dir);
    let session = store.create().expect("create");
    let router = test_router(store.clone());

    let cont = AutoContinueLoop::new(&session.id, "swarm-coordinator", "check git", 3);
    cont.begin_session(router.sessions()).expect("begin");

    let loaded = store.load(&session.id).expect("load");
    let persisted = SessionStore::chain_continuation(&loaded).expect("continuation");
    assert!(persisted.enabled);
    assert_eq!(persisted.active_agent, "swarm-coordinator");
    assert_eq!(persisted.max_steps, 3);

    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn continuation_finalized_after_plan_only_stop() {
    let dir = std::env::temp_dir().join(format!("rmng-cont-done-{}", uuid::Uuid::new_v4()));
    let store = SessionStore::new(&dir);
    let session = store.create().expect("create");
    let router = test_router(store.clone());

    let mut cont = AutoContinueLoop::new(&session.id, "swarm-coordinator", "just plan", 2);
    cont.begin_session(router.sessions()).expect("begin");

    let step = cont.run_step(&router).await.expect("step");
    assert!(matches!(step, AutoContinueStep::Stop { .. }));

    cont.finish_session(router.sessions(), "completed", ContinuationStatus::Done)
        .expect("finalize");

    let loaded = store.load(&session.id).expect("load");
    let orch = loaded.shared_context.get("orchestration").expect("orch");
    assert_eq!(orch.get("status").and_then(|v| v.as_str()), Some("completed"));
    assert_eq!(
        orch.get("awaiting_continuation").and_then(|v| v.as_bool()),
        Some(false)
    );
    assert!(orch.get("history").and_then(|v| v.as_array()).is_some());

    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn handoff_chain_sets_awaiting_continuation() {
    let dir = std::env::temp_dir().join(format!("rmng-cont-chain-{}", uuid::Uuid::new_v4()));
    let store = SessionStore::new(&dir);
    let session = store.create().expect("create");
    let router = test_router(store.clone());

    router
        .handoff_chain(
            &session.id,
            &[
                "swarm-coordinator".into(),
                "repo-keeper".into(),
                "runtime-executor".into(),
            ],
            "chain task",
            "continuation test",
        )
        .await
        .expect("chain");

    let loaded = store.load(&session.id).expect("load");
    let orch = loaded.shared_context.get("orchestration").expect("orch");
    assert_eq!(
        orch.get("awaiting_continuation").and_then(|v| v.as_bool()),
        Some(true)
    );
    assert_eq!(
        orch.get("continuation_agent").and_then(|v| v.as_str()),
        Some("runtime-executor")
    );

    let _ = std::fs::remove_dir_all(dir);
}
