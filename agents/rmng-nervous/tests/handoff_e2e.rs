//! Integration-style tests for L4→L3 handoff, session context, and write-back loop.

use rmng_core::{
    build_tool_result_record, daemon_running, persist_dispatch_to_session, send_intent_json,
    AuditLog, CoreIntent, HandleResponse, LlmConfig, LlmProvider, RmngConfig, SessionStore,
    ToolResult,
};
use rmng_nervous::{AgentRouter, RouteOutcome};
use std::io::{BufRead, BufReader};
use std::time::Instant;

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
async fn session_context_injected_into_prompt_assembly() {
    let dir = std::env::temp_dir().join(format!("rmng-ctx-{}", uuid::Uuid::new_v4()));
    let store = SessionStore::new(&dir);
    let mut session = store.create().expect("create");
    store
        .set_context(&mut session, "repo", serde_json::json!("RMNG-OS"))
        .expect("set context");
    let ctx = session.prompt_context();
    assert!(ctx.contains("RMNG-OS"));
    assert!(ctx.contains("session_id"));
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn l4_orchestrator_handoffs_to_l3_in_session() {
    let dir = std::env::temp_dir().join(format!("rmng-handoff-{}", uuid::Uuid::new_v4()));
    let store = SessionStore::new(&dir);
    let session = store.create().expect("create");
    let router = test_router(store);
    let outcome = router
        .ask_routed(Some(&session.id), "swarm-coordinator", "check git status")
        .await
        .expect("orchestrate");
    assert!(outcome.is_handoff());
    if let RouteOutcome::Handoff { to_agent, .. } = &outcome {
        assert_eq!(to_agent, "repo-keeper");
    }
    let loaded = router.sessions().load(&session.id).expect("load");
    assert_eq!(loaded.handoff_history.len(), 1);
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn explicit_handoff_cli_path_via_router() {
    let dir = std::env::temp_dir().join(format!("rmng-explicit-{}", uuid::Uuid::new_v4()));
    let store = SessionStore::new(&dir);
    let session = store.create().expect("create");
    let router = test_router(store);
    let outcome = router
        .handoff(
            &session.id,
            "swarm-coordinator",
            "repo-keeper",
            "check git status",
            "explicit delegation",
        )
        .await
        .expect("handoff");
    assert!(matches!(outcome, RouteOutcome::Handoff { .. }));
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn tool_result_written_to_shared_context_after_dispatch() {
    let dir = std::env::temp_dir().join(format!("rmng-writeback-{}", uuid::Uuid::new_v4()));
    let store = SessionStore::new(&dir);
    let session = store.create().expect("create");
    let router = test_router(store.clone());

    let outcome = router
        .handoff(
            &session.id,
            "swarm-coordinator",
            "repo-keeper",
            "check git status",
            "write-back test",
        )
        .await
        .expect("handoff");

    let mut intent = outcome.intent();
    AgentRouter::enrich_intent_metadata(&mut intent, Some(&session.id), Some("swarm-coordinator"));

    let resp = HandleResponse::core_success(
        "tool.execute:git.status",
        Some(ToolResult {
            success: true,
            output: "On branch main".into(),
            exit_code: Some(0),
        }),
    );
    persist_dispatch_to_session(&store, &session.id, &intent, &resp).expect("persist");

    let loaded = store.load(&session.id).expect("load");
    let ctx = loaded.prompt_context();
    assert!(ctx.contains("On branch main"));
    assert!(ctx.contains("git.status"));

    let results = loaded
        .shared_context
        .get("tool_results")
        .and_then(|v| v.as_array())
        .expect("tool_results");
    assert_eq!(results.len(), 1);
    assert!(results[0]["success"].as_bool().unwrap());

    // Next agent step sees prior tool output in prompt assembly.
    let next = router
        .ask_routed(Some(&session.id), "repo-keeper", "summarize last git status")
        .await
        .expect("follow-up ask");
    assert!(matches!(next, RouteOutcome::Direct { .. }));

    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn multi_hop_handoff_chain_records_full_history() {
    let dir = std::env::temp_dir().join(format!("rmng-chain-{}", uuid::Uuid::new_v4()));
    let store = SessionStore::new(&dir);
    let session = store.create().expect("create");
    let router = test_router(store);

    let chain = vec![
        "swarm-coordinator".into(),
        "repo-keeper".into(),
        "runtime-executor".into(),
    ];
    let outcome = router
        .handoff_chain(
            &session.id,
            &chain,
            "check git status",
            "L4→L3→L2 chain",
        )
        .await
        .expect("chain");

    if let RouteOutcome::Handoff { to_agent, .. } = &outcome {
        assert_eq!(to_agent, "runtime-executor");
    } else {
        panic!("expected final handoff outcome");
    }

    let loaded = router.sessions().load(&session.id).expect("load");
    assert_eq!(loaded.handoff_history.len(), 2);
    assert_eq!(loaded.handoff_history[0].to_agent, "repo-keeper");
    assert_eq!(loaded.handoff_history[1].to_agent, "runtime-executor");

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn build_tool_result_record_captures_mcp_and_metadata() {
    let intent = CoreIntent::McpProxy {
        mcp_server: "github".into(),
        mcp_tool: "search_issues".into(),
        mcp_args: serde_json::json!({"q": "rmng"}),
        metadata: Some(rmng_core::Metadata {
            trace_id: None,
            skill_name: None,
            session_id: Some("sid".into()),
            handoff_from: Some("research-curator".into()),
            handoff_to: None,
            handoff_chain: None,
        }),
    };
    let resp = HandleResponse::failure("mcp unavailable");
    let record = build_tool_result_record(&intent, &resp).expect("record");
    assert_eq!(record.tool, "github.search_issues");
    assert!(!record.success);
    assert_eq!(record.handoff_from.as_deref(), Some("research-curator"));
}

#[tokio::test]
async fn l4_handoff_dispatches_via_rmngd_when_running() {
    if !daemon_running() {
        eprintln!("skip: rmngd not running");
        return;
    }
    let store = SessionStore::default_store();
    let session = store.create().expect("create");
    let router = test_router(store.clone());
    let outcome = router
        .ask_routed(Some(&session.id), "swarm-coordinator", "check git status")
        .await
        .expect("orchestrate");

    let mut intent = outcome.intent();
    let handoff_from = match &outcome {
        RouteOutcome::Handoff { from_agent, .. } => Some(from_agent.as_str()),
        _ => None,
    };
    AgentRouter::enrich_intent_metadata(&mut intent, Some(&session.id), handoff_from);

    let json = serde_json::to_string(&intent).expect("serialize");
    let line = send_intent_json(&json).await.expect("send");
    assert!(!line.is_empty());

    let handle: HandleResponse = serde_json::from_str(line.trim()).expect("parse response");
    persist_dispatch_to_session(&store, &session.id, &intent, &handle).expect("write-back");

    let loaded = store.load(&session.id).expect("load session");
    let results = loaded
        .shared_context
        .get("tool_results")
        .and_then(|v| v.as_array());
    assert!(
        results.is_some_and(|r| !r.is_empty()),
        "tool_results should be written after rmngd dispatch"
    );

    let audit_path = AuditLog::default_path();
    let deadline = Instant::now();
    loop {
        let lines = tail_lines(&audit_path, 8);
        let ok = lines
            .iter()
            .any(|l| l.contains("git.status") && l.contains("ok"));
        if ok {
            break;
        }
        if deadline.elapsed().as_secs() > 8 {
            panic!("audit entry for git.status not found; last lines: {lines:?}");
        }
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }
}

fn tail_lines(path: &std::path::Path, n: usize) -> Vec<String> {
    let file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return Vec::new(),
    };
    let lines: Vec<String> = BufReader::new(file).lines().filter_map(|l| l.ok()).collect();
    lines.into_iter().rev().take(n).collect()
}