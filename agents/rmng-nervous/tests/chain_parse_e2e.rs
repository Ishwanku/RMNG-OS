//! Sprint 24: robust handoff metadata parsing.

use rmng_nervous::parse_core_intent;

#[test]
fn normalizes_comma_separated_handoff_chain() {
    let raw = r#"{"action":"plan.only","reasoning":"x","metadata":{"handoff_chain":"swarm-coordinator,repo-keeper"}}"#;
    let intent = parse_core_intent(raw).expect("parse");
    let chain = intent.metadata().unwrap().handoff_chain.as_ref().unwrap();
    assert_eq!(chain.len(), 2);
}

#[test]
fn extracts_json_from_surrounding_text() {
    let raw = r#"Sure! {"action":"plan.only","reasoning":"ok","metadata":{"session_id":"s1"}}"#;
    let intent = parse_core_intent(raw).expect("parse");
    assert!(matches!(intent, rmng_core::CoreIntent::PlanOnly { .. }));
}
