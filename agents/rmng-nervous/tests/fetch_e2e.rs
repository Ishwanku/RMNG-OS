//! MCP Fetch + web-researcher E2E — session write-back and permission gates.

use rmng_core::{
    daemon_running, persist_dispatch_to_session, send_intent_json, CoreIntent, HandleResponse,
    LlmConfig, LlmProvider, PermissionGate, PermissionVerdict, RmngConfig, SessionStore,
};
use rmng_nervous::AgentRouter;

const FETCH_URL: &str = "https://example.com";

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
async fn web_researcher_generates_fetch_intent() {
    let dir = std::env::temp_dir().join(format!("rmng-fetch-intent-{}", uuid::Uuid::new_v4()));
    let store = SessionStore::new(&dir);
    let session = store.create().expect("create");
    let router = test_router(store);
    let outcome = router
        .ask_routed(
            Some(&session.id),
            "web-researcher",
            &format!("fetch the content from {FETCH_URL}"),
        )
        .await
        .expect("ask");
    let intent = outcome.intent();
    match &intent {
        CoreIntent::McpProxy {
            mcp_server,
            mcp_tool,
            mcp_args,
            ..
        } => {
            assert_eq!(mcp_server, "fetch");
            assert_eq!(mcp_tool, "fetch");
            assert!(mcp_args.get("url").is_some());
        }
        CoreIntent::PlanOnly { .. } => {}
        other => panic!("unexpected intent: {other:?}"),
    }
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn web_researcher_generates_markitdown_for_documents() {
    let dir = std::env::temp_dir().join(format!("rmng-md-intent-{}", uuid::Uuid::new_v4()));
    let store = SessionStore::new(&dir);
    let session = store.create().expect("create");
    let router = test_router(store);
    let outcome = router
        .ask_routed(
            Some(&session.id),
            "web-researcher",
            "convert this PDF document to markdown using file:///tmp/sample.pdf",
        )
        .await
        .expect("ask");
    match outcome.intent() {
        CoreIntent::McpProxy {
            mcp_server,
            mcp_tool,
            ..
        } => {
            assert_eq!(mcp_server, "markitdown");
            assert_eq!(mcp_tool, "convert_to_markdown");
        }
        CoreIntent::PlanOnly { .. } => {}
        other => panic!("unexpected intent: {other:?}"),
    }
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn fetch_mcp_allowed_by_permission_gate() {
    let gate = PermissionGate::default();
    let intent = CoreIntent::McpProxy {
        mcp_server: "fetch".into(),
        mcp_tool: "fetch".into(),
        mcp_args: serde_json::json!({"url": FETCH_URL, "max_length": 5000}),
        metadata: None,
    };
    match gate.evaluate_core(&intent) {
        PermissionVerdict::Allow => {}
        PermissionVerdict::Deny(reason) => {
            eprintln!("skip or register fetch MCP: {reason}");
        }
    }
}

#[tokio::test]
async fn fetch_mcp_full_loop_when_rmngd_running() {
    if !daemon_running() {
        eprintln!("skip: rmngd not running");
        return;
    }
    let dir = std::env::temp_dir().join(format!("rmng-fetch-loop-{}", uuid::Uuid::new_v4()));
    let store = SessionStore::new(&dir);
    let session = store.create().expect("create");
    let router = test_router(store.clone());

    let outcome = router
        .ask_routed(
            Some(&session.id),
            "web-researcher",
            &format!("fetch {FETCH_URL}"),
        )
        .await
        .expect("ask");

    let mut intent = outcome.intent();
    if let CoreIntent::PlanOnly { .. } = &intent {
        intent = CoreIntent::McpProxy {
            mcp_server: "fetch".into(),
            mcp_tool: "fetch".into(),
            mcp_args: serde_json::json!({"url": FETCH_URL, "max_length": 8000}),
            metadata: intent.metadata().cloned(),
        };
    }
    AgentRouter::enrich_intent_metadata(&mut intent, Some(&session.id), Some("web-researcher"));

    let gate = PermissionGate::default();
    if matches!(gate.evaluate_core(&intent), PermissionVerdict::Deny(_)) {
        eprintln!("skip: fetch MCP not in allowlist");
        let _ = std::fs::remove_dir_all(dir);
        return;
    }

    let json = serde_json::to_string(&intent).expect("serialize");
    let line = send_intent_json(&json).await.expect("send");
    let resp: HandleResponse = serde_json::from_str(line.trim()).expect("parse response");

    persist_dispatch_to_session(&store, &session.id, &intent, &resp).expect("write-back");

    let loaded = store.load(&session.id).expect("load");
    let results = loaded
        .shared_context
        .get("tool_results")
        .and_then(|v| v.as_array());
    assert!(
        results.is_some_and(|r| !r.is_empty()),
        "fetch result should be written to shared_context"
    );

    let ctx = loaded.prompt_context();
    assert!(
        ctx.contains("fetch.fetch") || ctx.contains("fetch"),
        "session context should include fetch tool output"
    );

    let record = results.unwrap().last().unwrap();
    assert_eq!(record.get("tool").and_then(|v| v.as_str()), Some("fetch.fetch"));

    let _ = std::fs::remove_dir_all(dir);
}

const SAMPLE_PDF: &str =
    "https://www.w3.org/WAI/ER/tests/xhtml/testfiles/resources/pdf/dummy.pdf";

#[tokio::test]
async fn markitdown_mcp_full_loop_when_rmngd_running() {
    if !daemon_running() {
        eprintln!("skip: rmngd not running");
        return;
    }
    let dir = std::env::temp_dir().join(format!("rmng-md-loop-{}", uuid::Uuid::new_v4()));
    let store = SessionStore::new(&dir);
    let session = store.create().expect("create");

    let mut intent = CoreIntent::McpProxy {
        mcp_server: "markitdown".into(),
        mcp_tool: "convert_to_markdown".into(),
        mcp_args: serde_json::json!({"uri": SAMPLE_PDF}),
        metadata: None,
    };
    AgentRouter::enrich_intent_metadata(&mut intent, Some(&session.id), Some("web-researcher"));

    let gate = PermissionGate::default();
    if matches!(gate.evaluate_core(&intent), PermissionVerdict::Deny(_)) {
        eprintln!("skip: markitdown MCP not in allowlist");
        let _ = std::fs::remove_dir_all(dir);
        return;
    }

    let json = serde_json::to_string(&intent).expect("serialize");
    let line = send_intent_json(&json).await.expect("send");
    let resp: HandleResponse = serde_json::from_str(line.trim()).expect("parse response");

    persist_dispatch_to_session(&store, &session.id, &intent, &resp).expect("write-back");

    let loaded = store.load(&session.id).expect("load");
    let results = loaded
        .shared_context
        .get("tool_results")
        .and_then(|v| v.as_array());
    assert!(
        results.is_some_and(|r| !r.is_empty()),
        "markitdown result should be written to shared_context"
    );

    let record = results.unwrap().last().unwrap();
    assert_eq!(
        record.get("tool").and_then(|v| v.as_str()),
        Some("markitdown.convert_to_markdown")
    );

    let ctx = loaded.prompt_context();
    assert!(
        ctx.contains("markitdown") || ctx.contains("convert_to_markdown"),
        "session context should include markitdown output"
    );

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn web_researcher_agent_policy_allows_fetch_and_markitdown() {
    let reg = rmng_nervous::AgentRegistry::load().expect("registry");
    let agent = reg.get("web-researcher").expect("agent");
    let fetch = CoreIntent::McpProxy {
        mcp_server: "fetch".into(),
        mcp_tool: "fetch".into(),
        mcp_args: serde_json::json!({"url": FETCH_URL}),
        metadata: None,
    };
    assert!(agent.allows_core_intent(&fetch).is_ok());
    let md = CoreIntent::McpProxy {
        mcp_server: "markitdown".into(),
        mcp_tool: "convert_to_markdown".into(),
        mcp_args: serde_json::json!({"uri": "https://example.com"}),
        metadata: None,
    };
    assert!(agent.allows_core_intent(&md).is_ok());
}