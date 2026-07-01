//! Security isolation negative tests (Sprint 21–22).

use rmng_core::allowlist::{McpAllowlist, McpServerConfig};
use rmng_core::permission::{PermissionGate, PermissionVerdict};
use rmng_core::registry::IntegrationRegistry;
use rmng_core::{CoreIntent, IsolationLimits};
use rmng_mcp::{is_high_risk_mcp_server, normalize_profile, PROFILE_E2B, PROFILE_PLAYWRIGHT};
use std::collections::HashMap;

fn fixture_gate(servers: HashMap<String, McpServerConfig>) -> PermissionGate {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../integrations");
    let registry = IntegrationRegistry::load_from(root).expect("integrations");
    PermissionGate::from_registry(&registry).with_mcp_allowlist(McpAllowlist { servers })
}

fn high_risk_isolation(profile: &str) -> IsolationLimits {
    IsolationLimits {
        memory_mb: Some(512),
        pids_max: Some(64),
        new_session: true,
        cgroup: true,
        no_new_privs: true,
        seccomp_profile: Some(profile.into()),
        drop_capabilities: true,
        ..Default::default()
    }
}

#[test]
fn high_risk_servers_identified() {
    assert!(is_high_risk_mcp_server("playwright"));
    assert!(is_high_risk_mcp_server("e2b"));
    assert!(!is_high_risk_mcp_server("git"));
}

#[test]
fn seccomp_profiles_normalize() {
    assert_eq!(normalize_profile("playwright"), Some(PROFILE_PLAYWRIGHT));
    assert_eq!(normalize_profile("e2b"), Some(PROFILE_E2B));
    assert_eq!(normalize_profile("off"), None);
}

#[test]
fn seccomp_profile_makes_isolation_active() {
    let limits = IsolationLimits {
        seccomp_profile: Some("basic".into()),
        ..Default::default()
    };
    assert!(limits.is_active());
}

#[test]
fn e2b_denies_disallowed_tool_when_enabled() {
    let mut servers = HashMap::new();
    servers.insert(
        "e2b".into(),
        McpServerConfig {
            enabled: true,
            command: "npx".into(),
            args: vec!["-y".into(), "@e2b/mcp-server".into()],
            allowed_tools: vec!["run_code".into()],
            isolation: Some(high_risk_isolation("e2b")),
        },
    );
    let gate = fixture_gate(servers);
    let intent = CoreIntent::McpProxy {
        mcp_server: "e2b".into(),
        mcp_tool: "run_shell".into(),
        mcp_args: serde_json::json!({"cmd": "id"}),
        metadata: None,
    };
    assert!(matches!(gate.evaluate_core(&intent), PermissionVerdict::Deny(_)));
}

#[test]
fn playwright_denies_snapshot_when_not_allowlisted() {
    let mut servers = HashMap::new();
    servers.insert(
        "playwright".into(),
        McpServerConfig {
            enabled: true,
            command: "npx".into(),
            args: vec!["-y".into(), "@playwright/mcp@latest".into()],
            allowed_tools: vec!["browser_navigate".into()],
            isolation: Some(high_risk_isolation("playwright")),
        },
    );
    let gate = fixture_gate(servers);
    let intent = CoreIntent::McpProxy {
        mcp_server: "playwright".into(),
        mcp_tool: "browser_snapshot".into(),
        mcp_args: serde_json::json!({}),
        metadata: None,
    };
    assert!(matches!(gate.evaluate_core(&intent), PermissionVerdict::Deny(_)));
}

#[test]
fn mem0_delete_denied_for_repo_keeper_agent_policy() {
    let reg = rmng_nervous::AgentRegistry::load().expect("registry");
    let agent = reg.get("repo-keeper").expect("repo-keeper");
    let intent = CoreIntent::McpProxy {
        mcp_server: "mem0".into(),
        mcp_tool: "delete_memory".into(),
        mcp_args: serde_json::json!({"memory_id": "x"}),
        metadata: None,
    };
    assert!(agent.allows_core_intent(&intent).is_err());
}

#[test]
fn merge_isolation_keeps_seccomp_from_server_override() {
    let base = IsolationLimits {
        memory_mb: Some(512),
        seccomp_profile: None,
        ..Default::default()
    };
    let over = IsolationLimits {
        seccomp_profile: Some("e2b".into()),
        drop_capabilities: true,
        ..Default::default()
    };
    let m = IsolationLimits::merge(&base, Some(&over));
    assert_eq!(m.seccomp_profile.as_deref(), Some("e2b"));
    assert!(m.drop_capabilities);
}
