use super::{
    DENY_SCOPE, OPERATION_SPECS, OperationTransport, READ_SCOPE, SynapseAction, WRITE_SCOPE,
    required_scope_for_action, required_scope_for_parsed_action, scopes_satisfy,
};
use serde_json::json;

#[test]
fn canonical_operations_are_unique_and_fail_closed() {
    let mut names = std::collections::HashSet::new();
    for spec in OPERATION_SPECS {
        assert!(names.insert(spec.name), "duplicate operation {}", spec.name);
        assert!(
            spec.required_scope.is_some() || spec.name == "help",
            "only help may be public: {}",
            spec.name
        );
        if spec.transport == OperationTransport::Rest {
            assert!(super::is_rest_action(spec.name));
        }
    }
}

#[test]
fn read_scope_and_write_implies_read() {
    assert_eq!(required_scope_for_action("docker"), Some(READ_SCOPE));
    assert_eq!(required_scope_for_action("nodes"), Some(READ_SCOPE));
    assert!(scopes_satisfy(&[WRITE_SCOPE.into()], READ_SCOPE));
}

#[test]
fn parses_flux_actions() {
    match SynapseAction::from_flux_args(&json!({"action":"docker","subaction":"info"})).unwrap() {
        SynapseAction::FluxDocker(args) => assert_eq!(args.subaction, "info"),
        other => panic!("expected FluxDocker, got {other:?}"),
    }
    let logs = SynapseAction::from_flux_args(&json!({
        "action":"container",
        "subaction":"logs",
        "container_id":"abc",
        "lines":20
    }))
    .unwrap();
    match logs {
        SynapseAction::FluxContainer(args) => {
            assert_eq!(args.subaction, "logs");
            assert_eq!(args.container_id.as_deref(), Some("abc"));
            assert_eq!(args.lines, Some(20));
        }
        other => panic!("expected FluxContainer, got {other:?}"),
    }
}

#[test]
fn parsed_flux_subactions_require_write_for_mutating_ops() {
    let docker_info =
        SynapseAction::from_flux_args(&json!({"action":"docker","subaction":"info"})).unwrap();
    assert_eq!(
        required_scope_for_parsed_action(&docker_info),
        Some(READ_SCOPE)
    );

    let docker_prune =
        SynapseAction::from_flux_args(&json!({"action":"docker","subaction":"prune"})).unwrap();
    assert_eq!(
        required_scope_for_parsed_action(&docker_prune),
        Some(WRITE_SCOPE)
    );

    let container_stop =
        SynapseAction::from_flux_args(&json!({"action":"container","subaction":"stop"})).unwrap();
    assert_eq!(
        required_scope_for_parsed_action(&container_stop),
        Some(WRITE_SCOPE)
    );

    let compose_status =
        SynapseAction::from_flux_args(&json!({"action":"compose","subaction":"status"})).unwrap();
    assert_eq!(
        required_scope_for_parsed_action(&compose_status),
        Some(READ_SCOPE)
    );

    let compose_build =
        SynapseAction::from_flux_args(&json!({"action":"compose","subaction":"build"})).unwrap();
    assert_eq!(
        required_scope_for_parsed_action(&compose_build),
        Some(WRITE_SCOPE)
    );
}

#[test]
fn parsed_flux_unknown_subactions_fail_closed_for_scope() {
    let unknown =
        SynapseAction::from_flux_args(&json!({"action":"docker","subaction":"future"})).unwrap();
    assert_eq!(required_scope_for_parsed_action(&unknown), Some(DENY_SCOPE));
}

#[test]
fn parses_container_list_filters() {
    let action = SynapseAction::from_flux_args(&json!({
        "action": "container",
        "subaction": "list",
        "host": "dookie",
        "state": "running",
        "name_filter": "nginx",
        "image_filter": "nginx",
        "label_filter": "tier=edge",
        "response_format": "json"
    }))
    .unwrap();
    match action {
        SynapseAction::FluxContainer(args) => {
            assert_eq!(args.subaction, "list");
            assert_eq!(args.host.as_deref(), Some("dookie"));
            assert_eq!(args.state.as_deref(), Some("running"));
            assert_eq!(args.name_filter.as_deref(), Some("nginx"));
            assert_eq!(args.image_filter.as_deref(), Some("nginx"));
            assert_eq!(args.label_filter.as_deref(), Some("tier=edge"));
        }
        other => panic!("expected FluxContainer, got {other:?}"),
    }
}

#[test]
fn rejects_invalid_response_format_on_container() {
    let err = SynapseAction::from_flux_args(&json!({
        "action": "container",
        "subaction": "list",
        "response_format": "xml"
    }))
    .unwrap_err();
    assert!(err.to_string().contains("response_format") || err.to_string().contains("xml"));
}

#[test]
fn parses_scout_actions_and_rejects_missing_fields() {
    assert_eq!(
        SynapseAction::from_scout_args(&json!({"action":"nodes"})).unwrap(),
        SynapseAction::ScoutNodes
    );
    // exec now requires `command` (path is optional).
    let error =
        SynapseAction::from_scout_args(&json!({"action":"exec","host":"local"})).unwrap_err();
    assert!(
        error.to_string().contains("command"),
        "missing command field error expected, got: {error}"
    );
}
