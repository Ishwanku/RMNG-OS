//! Testing workflow E2E — run-tests / validate-output mock routing + agent policy.

use rmng_core::{
    CoreIntent, LlmConfig, LlmProvider, RmngConfig, SessionStore, ToolResultRecord,
};
use rmng_nervous::AgentRouter;

fn test_router(store: SessionStore) -> AgentRouter {
    let registry = rmng_nervous::AgentRegistry::load().expect("registry");
    let connector = rmng_nervous::NervousConnector::from_config(RmngConfig {
        llm: LlmConfig {
            llm_provider: LlmProvider::None,
            ..Default::default()
        },
        profile: None,
        profiles: vec![],
        ..Default::default()
    });
    AgentRouter::with_session_store(registry, connector, store)
}

#[tokio::test]
async fn repo_keeper_run_tests_generates_e2b_intent() {
    let dir = std::env::temp_dir().join(format!("rmng-run-tests-{}", uuid::Uuid::new_v4()));
    let store = SessionStore::new(&dir);
    let session = store.create().expect("create");
    let router = test_router(store);
    let outcome = router
        .ask_routed(Some(&session.id), "repo-keeper", "run tests in sandbox for smoke check")
        .await
        .expect("ask");
    match outcome.intent() {
        CoreIntent::McpProxy {
            mcp_server,
            mcp_tool,
            ..
        } => {
            assert_eq!(mcp_server, "e2b");
            assert_eq!(mcp_tool, "run_code");
        }
        other => panic!("unexpected intent: {other:?}"),
    }
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn research_curator_validate_output_plan_only() {
    let dir = std::env::temp_dir().join(format!("rmng-validate-out-{}", uuid::Uuid::new_v4()));
    let store = SessionStore::new(&dir);
    let mut session = store.create().expect("create");
    store
        .record_tool_result(
            &mut session,
            ToolResultRecord {
                timestamp: chrono::Utc::now(),
                tool: "e2b.run_code".into(),
                parameters: serde_json::json!({"code": "print(1)"}),
                output: r#"{"pass": true, "tests": 1}"#.into(),
                success: true,
                exit_code: Some(0),
                handoff_from: None,
                peak_rss_kb: None,
                cpu_time_ms: None,
                runtime_ms: None,
            },
        )
        .expect("record");
    let router = test_router(store);
    let outcome = router
        .ask_routed(
            Some(&session.id),
            "research-curator",
            "validate-output on the last sandbox run",
        )
        .await
        .expect("ask");
    match outcome.intent() {
        CoreIntent::PlanOnly { reasoning, .. } => {
            assert!(reasoning.contains("validate-output"));
            assert!(reasoning.contains("e2b.run_code"));
        }
        other => panic!("unexpected intent: {other:?}"),
    }
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn research_curator_regression_check_plan_only() {
    let dir = std::env::temp_dir().join(format!("rmng-regression-{}", uuid::Uuid::new_v4()));
    let store = SessionStore::new(&dir);
    let session = store.create().expect("create");
    let router = test_router(store);
    let outcome = router
        .ask_routed(
            Some(&session.id),
            "research-curator",
            "regression check against prior test results",
        )
        .await
        .expect("ask");
    match outcome.intent() {
        CoreIntent::PlanOnly { reasoning, .. } => {
            assert!(reasoning.contains("regression-check"));
        }
        other => panic!("unexpected intent: {other:?}"),
    }
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn testing_skills_present_on_sandbox_agents() {
    let reg = rmng_nervous::AgentRegistry::load().expect("registry");
    for id in ["repo-keeper", "research-curator"] {
        let agent = reg.get(id).expect(id);
        for skill in [
            "run-tests",
            "validate-output",
            "test-coverage-check",
            "regression-check",
        ] {
            assert!(agent.skills.iter().any(|s| s == skill), "{id} missing {skill}");
        }
    }
}
