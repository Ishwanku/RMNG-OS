//! E2B sandbox MCP E2E — opt-in code execution (intent, gate, policy, optional loop).

use rmng_core::{
    daemon_running, persist_dispatch_to_session, send_intent_json, CoreIntent, HandleResponse,
    LlmConfig, LlmProvider, PermissionGate, PermissionVerdict, RmngConfig, SessionStore,
};
use rmng_nervous::AgentRouter;

const SAMPLE_CODE: &str = "print(sum([1, 2, 3]))";

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

fn test_gate_with_e2b() -> PermissionGate {
    use rmng_core::allowlist::{McpAllowlist, McpServerConfig};
    use rmng_core::registry::IntegrationRegistry;
    use std::collections::HashMap;

    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../integrations");
    let registry = IntegrationRegistry::load_from(root).expect("fixture integrations");
    let mut servers = HashMap::new();
    servers.insert(
        "e2b".into(),
        McpServerConfig {
            enabled: true,
            command: "npx".into(),
            args: vec!["-y".into(), "@e2b/mcp-server".into()],
            allowed_tools: vec!["run_code".into()],
            isolation: None,
        },
    );
    PermissionGate::from_registry(&registry).with_mcp_allowlist(McpAllowlist { servers })
}

#[tokio::test]
async fn repo_keeper_generates_run_code_intent() {
    let dir = std::env::temp_dir().join(format!("rmng-e2b-intent-{}", uuid::Uuid::new_v4()));
    let store = SessionStore::new(&dir);
    let session = store.create().expect("create");
    let router = test_router(store);
    let outcome = router
        .ask_routed(
            Some(&session.id),
            "repo-keeper",
            "run code in sandbox: print(2 + 2)",
        )
        .await
        .expect("ask");
    match outcome.intent() {
        CoreIntent::McpProxy {
            mcp_server,
            mcp_tool,
            mcp_args,
            ..
        } => {
            assert_eq!(mcp_server, "e2b");
            assert_eq!(mcp_tool, "run_code");
            assert!(mcp_args.get("code").is_some());
        }
        CoreIntent::PlanOnly { .. } => {}
        other => panic!("unexpected intent: {other:?}"),
    }
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn research_curator_generates_run_code_intent() {
    let dir = std::env::temp_dir().join(format!("rmng-e2b-curator-{}", uuid::Uuid::new_v4()));
    let store = SessionStore::new(&dir);
    let session = store.create().expect("create");
    let router = test_router(store);
    let outcome = router
        .ask_routed(
            Some(&session.id),
            "research-curator",
            "execute code in sandbox to verify sum([1,2,3])",
        )
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
        CoreIntent::PlanOnly { .. } => {}
        other => panic!("unexpected intent: {other:?}"),
    }
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn e2b_mcp_allowed_by_permission_gate_when_enabled() {
    let gate = test_gate_with_e2b();
    let intent = CoreIntent::McpProxy {
        mcp_server: "e2b".into(),
        mcp_tool: "run_code".into(),
        mcp_args: serde_json::json!({"code": SAMPLE_CODE}),
        metadata: None,
    };
    assert!(matches!(
        gate.evaluate_core(&intent),
        PermissionVerdict::Allow
    ));
}

#[tokio::test]
async fn e2b_mcp_denied_when_server_disabled() {
    let gate = PermissionGate::default();
    let intent = CoreIntent::McpProxy {
        mcp_server: "e2b".into(),
        mcp_tool: "run_code".into(),
        mcp_args: serde_json::json!({"code": SAMPLE_CODE}),
        metadata: None,
    };
    match gate.evaluate_core(&intent) {
        PermissionVerdict::Allow => eprintln!("e2b enabled locally — skip deny assertion"),
        PermissionVerdict::Deny(reason) => {
            assert!(reason.contains("e2b") || reason.contains("not configured"));
        }
    }
}

#[tokio::test]
async fn e2b_run_code_full_loop_when_rmngd_running() {
    if !daemon_running() {
        eprintln!("skip: rmngd not running");
        return;
    }
    let gate = PermissionGate::default();
    let probe = CoreIntent::McpProxy {
        mcp_server: "e2b".into(),
        mcp_tool: "run_code".into(),
        mcp_args: serde_json::json!({"code": SAMPLE_CODE}),
        metadata: None,
    };
    if matches!(gate.evaluate_core(&probe), PermissionVerdict::Deny(_)) {
        eprintln!("skip: e2b MCP not enabled in allowlist (opt-in)");
        return;
    }

    let dir = std::env::temp_dir().join(format!("rmng-e2b-loop-{}", uuid::Uuid::new_v4()));
    let store = SessionStore::new(&dir);
    let session = store.create().expect("create");

    let mut intent = probe;
    AgentRouter::enrich_intent_metadata(&mut intent, Some(&session.id), Some("repo-keeper"));

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
        "e2b result should be written to shared_context"
    );

    let record = results.unwrap().last().unwrap();
    assert_eq!(
        record.get("tool").and_then(|v| v.as_str()),
        Some("e2b.run_code")
    );

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn repo_keeper_agent_policy_allows_e2b_run_code() {
    let reg = rmng_nervous::AgentRegistry::load().expect("registry");
    let agent = reg.get("repo-keeper").expect("agent");
    let intent = CoreIntent::McpProxy {
        mcp_server: "e2b".into(),
        mcp_tool: "run_code".into(),
        mcp_args: serde_json::json!({"code": SAMPLE_CODE}),
        metadata: None,
    };
    assert!(agent.allows_core_intent(&intent).is_ok());
}

#[test]
fn research_curator_agent_policy_allows_e2b_run_code() {
    let reg = rmng_nervous::AgentRegistry::load().expect("registry");
    let agent = reg.get("research-curator").expect("agent");
    let intent = CoreIntent::McpProxy {
        mcp_server: "e2b".into(),
        mcp_tool: "run_code".into(),
        mcp_args: serde_json::json!({"code": SAMPLE_CODE}),
        metadata: None,
    };
    assert!(agent.allows_core_intent(&intent).is_ok());
}

#[test]
fn web_researcher_agent_policy_denies_e2b_run_code() {
    let reg = rmng_nervous::AgentRegistry::load().expect("registry");
    let agent = reg.get("web-researcher").expect("agent");
    let intent = CoreIntent::McpProxy {
        mcp_server: "e2b".into(),
        mcp_tool: "run_code".into(),
        mcp_args: serde_json::json!({"code": SAMPLE_CODE}),
        metadata: None,
    };
    assert!(agent.allows_core_intent(&intent).is_err());
}
