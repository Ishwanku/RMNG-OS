//! Integration-style tests for L4→L3 handoff and session context.

use rmng_core::{daemon_running, send_intent_json, AuditLog, SessionStore};
use rmng_nervous::{AgentRouter, RouteOutcome};
use std::io::{BufRead, BufReader};
use std::time::Instant;

fn test_router(store: SessionStore) -> AgentRouter {
    let registry = rmng_nervous::AgentRegistry::load().expect("registry");
    AgentRouter::with_session_store(
        registry,
        rmng_nervous::NervousConnector::load(),
        store,
    )
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
async fn l4_handoff_dispatches_via_rmngd_when_running() {
    if !daemon_running() {
        eprintln!("skip: rmngd not running");
        return;
    }
    let store = SessionStore::default_store();
    let session = store.create().expect("create");
    let router = AgentRouter::load();
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
    let resp = send_intent_json(&json).await.expect("send");
    assert!(!resp.is_empty());

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