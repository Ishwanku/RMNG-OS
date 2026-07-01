//! Sprint 26 — daemon AutoContinueLoop wiring (in-process, no socket).

use rmng_core::{LlmConfig, LlmProvider, RmngConfig, Runtime, SessionStore};
use rmng_nervous::{AgentRouter, NervousConnector};
use rmngd::orchestration::DaemonOrchestrator;

fn mock_connector() -> NervousConnector {
    NervousConnector::from_config(RmngConfig {
        llm: LlmConfig {
            llm_provider: LlmProvider::None,
            ..Default::default()
        },
        profile: None,
        profiles: vec![],
        ..Default::default()
    })
}

#[tokio::test]
async fn daemon_continues_when_awaiting_continuation_set() {
    let dir = std::env::temp_dir().join(format!("rmng-daemon-cont-{}", uuid::Uuid::new_v4()));
    let store = SessionStore::new(&dir);
    let session = store.create().expect("create");
    let orch = DaemonOrchestrator::new(
        Runtime::bootstrap().unwrap_or_default(),
        AgentRouter::with_session_store(
            rmng_nervous::AgentRegistry::load().expect("registry"),
            mock_connector(),
            store.clone(),
        ),
    );

    {
        let mut loaded = store.load(&session.id).expect("load");
        store
            .set_orchestration_state(
                &mut loaded,
                serde_json::json!({
                    "status": "completed",
                    "awaiting_continuation": true,
                    "continuation_agent": "swarm-coordinator",
                }),
            )
            .expect("orch");
    }

    let resp = orch.continue_session(&session.id).await;
    assert!(resp.ok, "expected ok continue, got {:?}", resp.error);
    assert!(resp.steps_run >= 1 || resp.status == "completed");

    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn parse_daemon_line_recognizes_continue_action() {
    let json = r#"{"action":"orchestration.continue","session_id":"test-sid"}"#;
    let line = rmng_core::parse_daemon_line(json).expect("parse");
    match line {
        rmng_core::DaemonLine::OrchestrationContinue { session_id } => {
            assert_eq!(session_id, "test-sid");
        }
        _ => panic!("expected continue line"),
    }
}


#[tokio::test]
async fn should_trigger_continue_skips_failed_dispatch() {
    use rmng_core::{CoreIntent, HandleResponse};

    let dir = std::env::temp_dir().join(format!("rmng-daemon-trig-{}", uuid::Uuid::new_v4()));
    let store = SessionStore::new(&dir);
    let session = store.create().expect("create");
    {
        let mut loaded = store.load(&session.id).expect("load");
        store
            .set_orchestration_state(
                &mut loaded,
                serde_json::json!({
                    "status": "completed",
                    "awaiting_continuation": true,
                    "continuation_agent": "swarm-coordinator",
                }),
            )
            .expect("orch");
    }
    let orch = DaemonOrchestrator::new(
        Runtime::bootstrap().unwrap_or_default(),
        AgentRouter::with_session_store(
            rmng_nervous::AgentRegistry::load().expect("registry"),
            mock_connector(),
            store,
        ),
    );
    let intent = CoreIntent::ToolExecute {
        target: "git.status".into(),
        parameters: serde_json::json!({}),
        metadata: None,
    };
    let bad = HandleResponse::failure("tool failed");
    assert!(!orch.should_trigger_continue(&session.id, &intent, &bad));
    let good = HandleResponse::core_success("tool.execute:git.status", None);
    assert!(orch.should_trigger_continue(&session.id, &intent, &good));
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn continuation_state_survives_reload() {
    let dir = std::env::temp_dir().join(format!("rmng-daemon-reload-{}", uuid::Uuid::new_v4()));
    let store = SessionStore::new(&dir);
    let session = store.create().expect("create");
    {
        let mut loaded = store.load(&session.id).expect("load");
        store
            .set_orchestration_state(
                &mut loaded,
                serde_json::json!({
                    "status": "completed",
                    "awaiting_continuation": true,
                    "continuation_agent": "swarm-coordinator",
                    "continuation": {
                        "enabled": true,
                        "max_steps": 2,
                        "step": 1,
                        "start_agent": "swarm-coordinator",
                        "active_agent": "swarm-coordinator",
                        "next_prompt": "continue",
                        "status": "running"
                    }
                }),
            )
            .expect("orch");
    }
    let store2 = SessionStore::new(&dir);
    let loaded = store2.load(&session.id).expect("reload");
    let cont = SessionStore::chain_continuation(&loaded).expect("cont");
    assert_eq!(cont.step, 1);
    assert!(cont.should_run());
    let _ = std::fs::remove_dir_all(dir);
}
