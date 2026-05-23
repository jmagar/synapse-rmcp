use super::{required_scope_for_action, scopes_satisfy, SynapseAction, READ_SCOPE, WRITE_SCOPE};
use serde_json::json;

#[test]
fn read_scope_and_write_implies_read() {
    assert_eq!(required_scope_for_action("docker"), Some(READ_SCOPE));
    assert_eq!(required_scope_for_action("nodes"), Some(READ_SCOPE));
    assert!(scopes_satisfy(&[WRITE_SCOPE.into()], READ_SCOPE));
}

#[test]
fn parses_flux_actions() {
    assert_eq!(
        SynapseAction::from_flux_args(&json!({"action":"docker","subaction":"info"})).unwrap(),
        SynapseAction::FluxDocker {
            subaction: "info".into()
        }
    );
    assert_eq!(
        SynapseAction::from_flux_args(&json!({
            "action":"container",
            "subaction":"logs",
            "container_id":"abc",
            "lines":20
        }))
        .unwrap(),
        SynapseAction::FluxContainer {
            subaction: "logs".into(),
            container_id: Some("abc".into()),
            lines: Some(20),
        }
    );
}

#[test]
fn parses_scout_actions_and_rejects_missing_fields() {
    assert_eq!(
        SynapseAction::from_scout_args(&json!({"action":"nodes"})).unwrap(),
        SynapseAction::ScoutNodes
    );
    let error =
        SynapseAction::from_scout_args(&json!({"action":"exec","host":"local"})).unwrap_err();
    assert!(error.to_string().contains("path"));
}
