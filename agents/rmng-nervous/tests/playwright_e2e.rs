//! Playwright MCP E2E — opt-in browser interaction (intent, gate, policy, navigation loop).

use rmng_core::{
    daemon_running, persist_dispatch_to_session, send_intent_json, CoreIntent, HandleResponse,
    LlmConfig, LlmProvider, PermissionGate, PermissionVerdict, RmngConfig, SessionStore,
};
use rmng_nervous::AgentRouter;

const NAV_URL: &str = "https://example.com";

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

fn test_gate_with_playwright() -> PermissionGate {
    use rmng_core::allowlist::{McpAllowlist, McpServerConfig};
    use rmng_core::registry::IntegrationRegistry;
    use std::collections::HashMap;

    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../integrations");
    let registry = IntegrationRegistry::load_from(root).expect("fixture integrations");
    let mut servers = HashMap::new();
    servers.insert(
        "playwright".into(),
        McpServerConfig {
            enabled: true,
            command: "npx".into(),
            args: vec!["-y".into(), "@playwright/mcp@latest".into()],
            allowed_tools: vec![
                "browser_navigate".into(),
                "browser_snapshot".into(),
                "browser_click".into(),
            ],
            isolation: None,
        },
    );
    PermissionGate::from_registry(&registry).with_mcp_allowlist(McpAllowlist { servers })
}

#[tokio::test]
async fn browser_researcher_generates_navigate_intent() {
    let dir = std::env::temp_dir().join(format!("rmng-pw-intent-{}", uuid::Uuid::new_v4()));
    let store = SessionStore::new(&dir);
    let session = store.create().expect("create");
    let router = test_router(store);
    let outcome = router
        .ask_routed(
            Some(&session.id),
            "browser-researcher",
            &format!("navigate browser to {NAV_URL}"),
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
            assert_eq!(mcp_server, "playwright");
            assert_eq!(mcp_tool, "browser_navigate");
            assert!(mcp_args.get("url").is_some());
        }
        CoreIntent::PlanOnly { .. } => {}
        other => panic!("unexpected intent: {other:?}"),
    }
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn playwright_mcp_allowed_by_permission_gate_when_enabled() {
    let gate = test_gate_with_playwright();
    let intent = CoreIntent::McpProxy {
        mcp_server: "playwright".into(),
        mcp_tool: "browser_navigate".into(),
        mcp_args: serde_json::json!({"url": NAV_URL}),
        metadata: None,
    };
    assert!(matches!(
        gate.evaluate_core(&intent),
        PermissionVerdict::Allow
    ));
}

#[tokio::test]
async fn playwright_mcp_denied_when_server_disabled() {
    let gate = PermissionGate::default();
    let intent = CoreIntent::McpProxy {
        mcp_server: "playwright".into(),
        mcp_tool: "browser_navigate".into(),
        mcp_args: serde_json::json!({"url": NAV_URL}),
        metadata: None,
    };
    match gate.evaluate_core(&intent) {
        PermissionVerdict::Allow => {
            eprintln!("playwright enabled in local allowlist — skip deny assertion");
        }
        PermissionVerdict::Deny(reason) => {
            assert!(
                reason.contains("playwright") || reason.contains("not configured"),
                "expected disabled/unconfigured: {reason}"
            );
        }
    }
}

#[tokio::test]
async fn playwright_mcp_full_loop_when_rmngd_running() {
    if !daemon_running() {
        eprintln!("skip: rmngd not running");
        return;
    }
    let gate = PermissionGate::default();
    let probe = CoreIntent::McpProxy {
        mcp_server: "playwright".into(),
        mcp_tool: "browser_navigate".into(),
        mcp_args: serde_json::json!({"url": NAV_URL}),
        metadata: None,
    };
    if matches!(gate.evaluate_core(&probe), PermissionVerdict::Deny(_)) {
        eprintln!("skip: playwright MCP not enabled in allowlist (opt-in)");
        return;
    }

    let dir = std::env::temp_dir().join(format!("rmng-pw-loop-{}", uuid::Uuid::new_v4()));
    let store = SessionStore::new(&dir);
    let session = store.create().expect("create");

    let mut intent = CoreIntent::McpProxy {
        mcp_server: "playwright".into(),
        mcp_tool: "browser_navigate".into(),
        mcp_args: serde_json::json!({"url": NAV_URL}),
        metadata: None,
    };
    AgentRouter::enrich_intent_metadata(&mut intent, Some(&session.id), Some("browser-researcher"));

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
        "playwright result should be written to shared_context"
    );

    let record = results.unwrap().last().unwrap();
    assert_eq!(
        record.get("tool").and_then(|v| v.as_str()),
        Some("playwright.browser_navigate")
    );

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn browser_researcher_agent_policy_allows_playwright_tools() {
    let reg = rmng_nervous::AgentRegistry::load().expect("registry");
    let agent = reg.get("browser-researcher").expect("agent");
    for tool in ["browser_navigate", "browser_snapshot", "browser_click"] {
        let intent = CoreIntent::McpProxy {
            mcp_server: "playwright".into(),
            mcp_tool: tool.into(),
            mcp_args: serde_json::json!({"url": NAV_URL}),
            metadata: None,
        };
        assert!(
            agent.allows_core_intent(&intent).is_ok(),
            "agent should allow playwright:{tool}"
        );
    }
    let fetch = CoreIntent::McpProxy {
        mcp_server: "fetch".into(),
        mcp_tool: "fetch".into(),
        mcp_args: serde_json::json!({"url": NAV_URL}),
        metadata: None,
    };
    assert!(agent.allows_core_intent(&fetch).is_err());
}