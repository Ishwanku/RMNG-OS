//! Sprint 24-25: robust handoff metadata parsing.

use rmng_nervous::parse_core_intent;

#[test]
fn normalizes_comma_separated_handoff_chain() {
    let raw = r#"{"action":"plan.only","reasoning":"x","metadata":{"handoff_chain":"swarm-coordinator,repo-keeper"}}"#;
    let intent = parse_core_intent(raw).expect("parse");
    let chain = intent.metadata().unwrap().handoff_chain.as_ref().unwrap();
    assert_eq!(chain.len(), 2);
}

#[test]
fn normalizes_arrow_handoff_chain() {
    let raw = r#"{"action":"plan.only","reasoning":"x","metadata":{"handoff_chain":"swarm-coordinator->repo-keeper->runtime-executor"}}"#;
    let intent = parse_core_intent(raw).expect("parse");
    assert_eq!(intent.metadata().unwrap().handoff_chain.as_ref().unwrap().len(), 3);
}

#[test]
fn normalizes_hop_failure_policy() {
    let raw = r#"{"action":"plan.only","reasoning":"x","metadata":{"hop_failure_policy":"SKIP"}}"#;
    let intent = parse_core_intent(raw).expect("parse");
    assert_eq!(
        intent.metadata().unwrap().hop_failure_policy.as_deref(),
        Some("skip")
    );
}

#[test]
fn extracts_json_from_surrounding_text() {
    let raw = r#"Sure! {"action":"plan.only","reasoning":"ok","metadata":{"session_id":"s1"}}"#;
    let intent = parse_core_intent(raw).expect("parse");
    assert!(matches!(intent, rmng_core::CoreIntent::PlanOnly { .. }));
}

#[test]
fn hoists_top_level_handoff_chain() {
    let raw = r#"{"action":"plan_only","reasoning":"x","handoff_chain":["swarm-coordinator","repo-keeper"]}"#;
    let intent = parse_core_intent(raw).expect("parse");
    assert_eq!(intent.metadata().unwrap().handoff_chain.as_ref().unwrap().len(), 2);
}

#[test]
fn normalizes_semicolon_separated_handoff_chain() {
    let raw = r#"{"action":"plan.only","reasoning":"x","metadata":{"handoff_chain":"swarm-coordinator;repo-keeper;runtime-executor"}}"#;
    let intent = parse_core_intent(raw).expect("parse");
    assert_eq!(intent.metadata().unwrap().handoff_chain.as_ref().unwrap().len(), 3);
}

#[test]
fn drops_single_agent_handoff_chain() {
    let raw = r#"{"action":"plan.only","reasoning":"x","metadata":{"handoff_chain":["repo-keeper"]}}"#;
    let intent = parse_core_intent(raw).expect("parse");
    assert!(intent.metadata().unwrap().handoff_chain.is_none());
}

#[test]
fn filters_empty_strings_in_handoff_chain_array() {
    let raw = r#"{"action":"plan.only","reasoning":"x","metadata":{"handoff_chain":["swarm-coordinator","","repo-keeper"]}}"#;
    let intent = parse_core_intent(raw).expect("parse");
    let chain = intent.metadata().unwrap().handoff_chain.as_ref().unwrap();
    assert_eq!(chain.len(), 2);
}
