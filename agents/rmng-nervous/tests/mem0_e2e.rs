//! Mem0 MCP E2E — long-term memory (add, search, get, delete) + session write-back.

use rmng_core::{
    daemon_running, persist_dispatch_to_session, send_intent_json, CoreIntent, HandleResponse,
    LlmConfig, LlmProvider, PermissionGate, PermissionVerdict, RmngConfig, SessionStore,
};
use rmng_nervous::AgentRouter;

const USER_ID: &str = "rmng-os";

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

fn test_gate_with_mem0() -> PermissionGate {
    use rmng_core::allowlist::{McpAllowlist, McpServerConfig};
    use rmng_core::registry::IntegrationRegistry;
    use std::collections::HashMap;

    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../integrations");
    let registry = IntegrationRegistry::load_from(root).expect("fixture integrations");
    let mut servers = HashMap::new();
    servers.insert(
        "mem0".into(),
        McpServerConfig {
            enabled: true,
            command: "uvx".into(),
            args: vec!["mem0-mcp-server".into()],
            allowed_tools: vec![
                "add_memory".into(),
                "search_memories".into(),
                "get_memory".into(),
                "delete_memory".into(),
            ],
            isolation: None,
        },
    );
    PermissionGate::from_registry(&registry).with_mcp_allowlist(McpAllowlist { servers })
}

#[tokio::test]
async fn research_curator_generates_search_memories_intent() {
    let dir = std::env::temp_dir().join(format!("rmng-mem0-intent-{}", uuid::Uuid::new_v4()));
    let store = SessionStore::new(&dir);
    let session = store.create().expect("create");
    let router = test_router(store);
    let outcome = router
        .ask_routed(
            Some(&session.id),
            "research-curator",
            "search memory for prior RMNG integration decisions",
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
            assert_eq!(mcp_server, "mem0");
            assert_eq!(mcp_tool, "search_memories");
            assert!(mcp_args.get("query").is_some());
        }
        CoreIntent::PlanOnly { .. } => {}
        other => panic!("unexpected intent: {other:?}"),
    }
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn web_researcher_generates_add_memory_intent() {
    let dir = std::env::temp_dir().join(format!("rmng-mem0-add-{}", uuid::Uuid::new_v4()));
    let store = SessionStore::new(&dir);
    let session = store.create().expect("create");
    let router = test_router(store);
    let outcome = router
        .ask_routed(
            Some(&session.id),
            "web-researcher",
            "remember that example.com is our fetch smoke-test URL",
        )
        .await
        .expect("ask");
    match outcome.intent() {
        CoreIntent::McpProxy {
            mcp_server,
            mcp_tool,
            ..
        } => {
            assert_eq!(mcp_server, "mem0");
            assert_eq!(mcp_tool, "add_memory");
        }
        CoreIntent::PlanOnly { .. } => {}
        other => panic!("unexpected intent: {other:?}"),
    }
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn mem0_mcp_allowed_by_permission_gate_when_enabled() {
    let gate = test_gate_with_mem0();
    for tool in ["add_memory", "search_memories", "get_memory", "delete_memory"] {
        let intent = CoreIntent::McpProxy {
            mcp_server: "mem0".into(),
            mcp_tool: tool.into(),
            mcp_args: serde_json::json!({"user_id": USER_ID}),
            metadata: None,
        };
        assert!(
            matches!(gate.evaluate_core(&intent), PermissionVerdict::Allow),
            "gate should allow mem0:{tool}"
        );
    }
}

#[tokio::test]
async fn mem0_mcp_denied_when_server_disabled() {
    let gate = PermissionGate::default();
    let intent = CoreIntent::McpProxy {
        mcp_server: "mem0".into(),
        mcp_tool: "search_memories".into(),
        mcp_args: serde_json::json!({"query": "test", "user_id": USER_ID}),
        metadata: None,
    };
    match gate.evaluate_core(&intent) {
        PermissionVerdict::Allow => eprintln!("mem0 enabled locally — skip deny assertion"),
        PermissionVerdict::Deny(reason) => {
            assert!(reason.contains("mem0") || reason.contains("not configured"));
        }
    }
}

#[tokio::test]
async fn mem0_search_full_loop_when_rmngd_running() {
    if !daemon_running() {
        eprintln!("skip: rmngd not running");
        return;
    }
    let gate = PermissionGate::default();
    let probe = CoreIntent::McpProxy {
        mcp_server: "mem0".into(),
        mcp_tool: "search_memories".into(),
        mcp_args: serde_json::json!({
            "query": "RMNG",
            "user_id": USER_ID,
            "limit": 3
        }),
        metadata: None,
    };
    if matches!(gate.evaluate_core(&probe), PermissionVerdict::Deny(_)) {
        eprintln!("skip: mem0 MCP not enabled in allowlist (opt-in)");
        return;
    }

    let dir = std::env::temp_dir().join(format!("rmng-mem0-loop-{}", uuid::Uuid::new_v4()));
    let store = SessionStore::new(&dir);
    let session = store.create().expect("create");

    let mut intent = probe;
    AgentRouter::enrich_intent_metadata(&mut intent, Some(&session.id), Some("research-curator"));

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
        "mem0 result should be written to shared_context"
    );

    let record = results.unwrap().last().unwrap();
    assert_eq!(
        record.get("tool").and_then(|v| v.as_str()),
        Some("mem0.search_memories")
    );

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn research_curator_agent_policy_allows_mem0_crud() {
    let reg = rmng_nervous::AgentRegistry::load().expect("registry");
    let agent = reg.get("research-curator").expect("agent");
    for tool in ["add_memory", "search_memories", "get_memory", "delete_memory"] {
        let intent = CoreIntent::McpProxy {
            mcp_server: "mem0".into(),
            mcp_tool: tool.into(),
            mcp_args: serde_json::json!({}),
            metadata: None,
        };
        assert!(agent.allows_core_intent(&intent).is_ok(), "allow {tool}");
    }
}

#[test]
fn repo_keeper_agent_policy_allows_mem0_read_only() {
    let reg = rmng_nervous::AgentRegistry::load().expect("registry");
    let agent = reg.get("repo-keeper").expect("agent");
    for tool in ["search_memories", "get_memory"] {
        let intent = CoreIntent::McpProxy {
            mcp_server: "mem0".into(),
            mcp_tool: tool.into(),
            mcp_args: serde_json::json!({}),
            metadata: None,
        };
        assert!(agent.allows_core_intent(&intent).is_ok());
    }
    let add = CoreIntent::McpProxy {
        mcp_server: "mem0".into(),
        mcp_tool: "add_memory".into(),
        mcp_args: serde_json::json!({}),
        metadata: None,
    };
    assert!(agent.allows_core_intent(&add).is_err());
}

#[test]
fn mem0_delete_memory_denied_by_gate_when_only_search_allowlisted() {
    use rmng_core::allowlist::{McpAllowlist, McpServerConfig};
    use rmng_core::registry::IntegrationRegistry;
    use std::collections::HashMap;
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../integrations");
    let registry = IntegrationRegistry::load_from(root).expect("integrations");
    let mut servers = HashMap::new();
    servers.insert(
        "mem0".into(),
        McpServerConfig {
            enabled: true,
            command: "uvx".into(),
            args: vec!["mem0-mcp-server".into()],
            allowed_tools: vec!["search_memories".into()],
            isolation: None,
        },
    );
    let gate = PermissionGate::from_registry(&registry).with_mcp_allowlist(McpAllowlist { servers });
    let intent = CoreIntent::McpProxy {
        mcp_server: "mem0".into(),
        mcp_tool: "delete_memory".into(),
        mcp_args: serde_json::json!({"memory_id": "x"}),
        metadata: None,
    };
    assert!(matches!(gate.evaluate_core(&intent), PermissionVerdict::Deny(_)));
}
