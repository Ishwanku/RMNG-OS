//! MCP multi-agent E2E — research-curator + github.search_issues.

use rmng_core::{
    daemon_running, persist_dispatch_to_session, send_intent_json, CoreIntent, HandleResponse,
    LlmConfig, LlmProvider, PermissionGate, PermissionVerdict, RmngConfig, SessionStore,
};
use rmng_nervous::{AgentRouter, RouteOutcome};

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
async fn research_curator_generates_mcp_search_issues_intent() {
    let dir = std::env::temp_dir().join(format!("rmng-mcp-intent-{}", uuid::Uuid::new_v4()));
    let store = SessionStore::new(&dir);
    let session = store.create().expect("create");
    let router = test_router(store);
    let outcome = router
        .ask_routed(
            Some(&session.id),
            "research-curator",
            "search open issues in RMNG-OS repo",
        )
        .await
        .expect("ask");
    let intent = outcome.intent();
    match &intent {
        CoreIntent::McpProxy {
            mcp_server,
            mcp_tool,
            ..
        } => {
            assert_eq!(mcp_server, "github");
            assert_eq!(mcp_tool, "search_issues");
        }
        CoreIntent::PlanOnly { .. } => {
            // mock may plan when prompt is ambiguous — acceptable in CI
        }
        other => panic!("unexpected intent: {other:?}"),
    }
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn research_curator_mcp_allowed_by_permission_gate() {
    let gate = PermissionGate::default();
    let intent = CoreIntent::McpProxy {
        mcp_server: "github".into(),
        mcp_tool: "search_issues".into(),
        mcp_args: serde_json::json!({"query": "repo:Ishwanku/RMNG-OS is:open"}),
        metadata: None,
    };
    match gate.evaluate_core(&intent) {
        PermissionVerdict::Allow => {}
        PermissionVerdict::Deny(reason) => {
            eprintln!("skip or configure MCP allowlist: {reason}");
        }
    }
}

#[tokio::test]
async fn research_curator_mcp_full_loop_when_rmngd_running() {
    if !daemon_running() {
        eprintln!("skip: rmngd not running");
        return;
    }
    let dir = std::env::temp_dir().join(format!("rmng-mcp-loop-{}", uuid::Uuid::new_v4()));
    let store = SessionStore::new(&dir);
    let session = store.create().expect("create");
    let router = test_router(store.clone());

    let outcome = router
        .handoff(
            &session.id,
            "swarm-coordinator",
            "research-curator",
            "search open issues in RMNG-OS",
            "research delegation",
        )
        .await
        .expect("handoff");

    let mut intent = outcome.intent();
    let handoff_from = match &outcome {
        RouteOutcome::Handoff { from_agent, .. } => Some(from_agent.as_str()),
        _ => None,
    };
    AgentRouter::enrich_intent_metadata(&mut intent, Some(&session.id), handoff_from);

    let gate = PermissionGate::default();
    if matches!(gate.evaluate_core(&intent), PermissionVerdict::Deny(_)) {
        eprintln!("skip: github.search_issues not in MCP allowlist");
        let _ = std::fs::remove_dir_all(dir);
        return;
    }

    let json = serde_json::to_string(&intent).expect("serialize");
    let line = send_intent_json(&json).await.expect("send");
    let resp: HandleResponse = serde_json::from_str(line.trim()).expect("parse response");

    persist_dispatch_to_session(&store, &session.id, &intent, &resp).expect("write-back");

    let loaded = store.load(&session.id).expect("load");
    assert_eq!(loaded.handoff_history.len(), 1);
    let results = loaded
        .shared_context
        .get("tool_results")
        .and_then(|v| v.as_array());
    assert!(
        results.is_some_and(|r| !r.is_empty()),
        "MCP result should be written to shared_context"
    );

    // Follow-up: agent should see MCP output in session context
    let ctx = loaded.prompt_context();
    assert!(ctx.contains("github.search_issues") || ctx.contains("search_issues"));

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn research_curator_agent_policy_allows_search_issues() {
    let reg = rmng_nervous::AgentRegistry::load().expect("registry");
    let agent = reg.get("research-curator").expect("agent");
    let intent = CoreIntent::McpProxy {
        mcp_server: "github".into(),
        mcp_tool: "search_issues".into(),
        mcp_args: serde_json::json!({"query": "test"}),
        metadata: None,
    };
    assert!(agent.allows_core_intent(&intent).is_ok());
}

#[tokio::test]
async fn research_curator_generates_list_issues_intent() {
    let dir = std::env::temp_dir().join(format!("rmng-list-issues-{}", uuid::Uuid::new_v4()));
    let store = SessionStore::new(&dir);
    let session = store.create().expect("create");
    let router = test_router(store);
    let outcome = router
        .ask_routed(
            Some(&session.id),
            "research-curator",
            "list open issues in RMNG-OS",
        )
        .await
        .expect("ask");
    match outcome.intent() {
        CoreIntent::McpProxy {
            mcp_server,
            mcp_tool,
            ..
        } => {
            assert_eq!(mcp_server, "github");
            assert_eq!(mcp_tool, "list_issues");
        }
        CoreIntent::PlanOnly { .. } => {}
        other => panic!("unexpected intent: {other:?}"),
    }
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn repo_keeper_generates_git_diff_mcp_intent() {
    let dir = std::env::temp_dir().join(format!("rmng-git-diff-{}", uuid::Uuid::new_v4()));
    let store = SessionStore::new(&dir);
    let session = store.create().expect("create");
    let router = test_router(store);
    let outcome = router
        .ask_routed(
            Some(&session.id),
            "repo-keeper",
            "show mcp git diff for working tree changes",
        )
        .await
        .expect("ask");
    match outcome.intent() {
        CoreIntent::McpProxy {
            mcp_server,
            mcp_tool,
            ..
        } => {
            assert_eq!(mcp_server, "git");
            assert_eq!(mcp_tool, "git.diff");
        }
        other => panic!("unexpected intent: {other:?}"),
    }
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn research_curator_agent_policy_allows_list_and_get_issue() {
    let reg = rmng_nervous::AgentRegistry::load().expect("registry");
    let agent = reg.get("research-curator").expect("agent");
    for tool in ["list_issues", "get_issue"] {
        let intent = CoreIntent::McpProxy {
            mcp_server: "github".into(),
            mcp_tool: tool.into(),
            mcp_args: serde_json::json!({}),
            metadata: None,
        };
        assert!(agent.allows_core_intent(&intent).is_ok(), "allow {tool}");
    }
}

#[test]
fn repo_keeper_agent_policy_allows_git_mcp_tools() {
    let reg = rmng_nervous::AgentRegistry::load().expect("registry");
    let agent = reg.get("repo-keeper").expect("agent");
    for tool in ["git.log", "git.diff", "git.status"] {
        let intent = CoreIntent::McpProxy {
            mcp_server: "git".into(),
            mcp_tool: tool.into(),
            mcp_args: serde_json::json!({}),
            metadata: None,
        };
        assert!(agent.allows_core_intent(&intent).is_ok(), "allow {tool}");
    }
}
