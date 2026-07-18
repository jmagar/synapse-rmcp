use super::{ALL_URIS, all_resources};

#[test]
fn list_resources_returns_all_eight_uris() {
    let resources = all_resources();
    assert_eq!(resources.len(), 8, "should expose exactly 8 resource URIs");

    let uris: Vec<String> = resources.iter().map(|r| r.uri.clone()).collect();
    for expected_uri in ALL_URIS {
        assert!(
            uris.contains(&expected_uri.to_string()),
            "URI {expected_uri} should be in the resource list; got: {uris:?}"
        );
    }
}

#[test]
fn all_resources_have_mime_types() {
    for resource in all_resources() {
        assert!(
            resource.mime_type.is_some(),
            "resource {} should have a mime_type",
            resource.uri
        );
    }
}

#[test]
fn all_resources_have_descriptions() {
    for resource in all_resources() {
        assert!(
            resource.description.is_some(),
            "resource {} should have a description",
            resource.uri
        );
    }
}

#[test]
fn schema_resources_have_json_mime_type() {
    let resources = all_resources();
    for resource in &resources {
        if resource.uri.contains("/schema/") {
            assert_eq!(
                resource.mime_type.as_deref(),
                Some("application/json"),
                "schema resource should have application/json mime type"
            );
        }
    }
}

#[test]
fn help_resources_have_markdown_mime_type() {
    let resources = all_resources();
    for resource in &resources {
        if resource.uri.contains("/help/") {
            assert_eq!(
                resource.mime_type.as_deref(),
                Some("text/markdown"),
                "help resource should have text/markdown mime type"
            );
        }
    }
}

#[tokio::test]
async fn read_hosts_resource_returns_json_array() {
    let state = crate::testing::loopback_state();
    let contents = super::read_resource(super::URI_HOSTS, &state)
        .await
        .expect("read_resource hosts should succeed");

    let text = match &contents {
        rmcp::model::ResourceContents::TextResourceContents { text, .. } => text.clone(),
        _ => panic!("hosts resource should be text"),
    };
    let parsed: serde_json::Value =
        serde_json::from_str(&text).expect("hosts resource should be valid JSON");
    assert!(parsed.is_array(), "hosts resource should be a JSON array");
    // loopback_state uses a default host repo, which includes at least "local"
    let arr = parsed.as_array().unwrap();
    assert!(!arr.is_empty(), "hosts array should not be empty");
}

#[tokio::test]
async fn read_schema_resource_returns_json() {
    let state = crate::testing::loopback_state();
    let contents = super::read_resource(super::URI_SCHEMA_FLUX, &state)
        .await
        .expect("read_resource schema/flux should succeed");

    let text = match &contents {
        rmcp::model::ResourceContents::TextResourceContents { text, .. } => text.clone(),
        _ => panic!("schema resource should be text"),
    };
    let parsed: serde_json::Value =
        serde_json::from_str(&text).expect("schema resource should be valid JSON");
    assert_eq!(parsed["name"], "flux");
    assert!(
        parsed.is_object(),
        "schema resource should be one tool definition"
    );
}

#[tokio::test]
async fn status_and_activity_resources_share_runtime_state() {
    let state = crate::testing::loopback_state();
    state.activity.record("rest", "scout.nodes", true, None);
    for (uri, expected) in [
        (super::URI_STATUS, "status"),
        (super::URI_ACTIVITY, "events"),
    ] {
        let contents = super::read_resource(uri, &state).await.unwrap();
        let rmcp::model::ResourceContents::TextResourceContents { text, .. } = contents else {
            panic!("expected text resource");
        };
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert!(parsed.get(expected).is_some());
    }
}

#[test]
fn only_live_resources_require_read_scope() {
    assert!(super::requires_read_scope(super::URI_HOSTS));
    assert!(super::requires_read_scope(super::URI_COMPOSE_PROJECTS));
    assert!(super::requires_read_scope(super::URI_STATUS));
    assert!(super::requires_read_scope(super::URI_ACTIVITY));
    assert!(!super::requires_read_scope(super::URI_SCHEMA_FLUX));
    assert!(!super::requires_read_scope(super::URI_HELP_FLUX));
}

#[tokio::test]
async fn read_help_flux_resource_returns_markdown() {
    let state = crate::testing::loopback_state();
    let contents = super::read_resource(super::URI_HELP_FLUX, &state)
        .await
        .expect("read_resource help/flux should succeed");

    let text = match &contents {
        rmcp::model::ResourceContents::TextResourceContents { text, .. } => text.clone(),
        _ => panic!("help resource should be text"),
    };
    assert!(
        text.contains("# flux tool help"),
        "help text should have a heading"
    );
    assert!(!text.is_empty(), "help text should not be empty");
}

#[tokio::test]
async fn read_unknown_resource_returns_error() {
    let state = crate::testing::loopback_state();
    let err = super::read_resource("synapse://unknown/resource", &state)
        .await
        .expect_err("unknown resource should return an error");
    assert!(
        err.to_string().contains("unknown resource"),
        "error should describe the unknown URI"
    );
}
