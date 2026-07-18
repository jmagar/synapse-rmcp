//! Route-level smoke tests for public status and optional REST compatibility.

use axum::{
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode, header},
};
use serde_json::{Value, json};
use synapse2::{server, testing::loopback_state};
use tower::ServiceExt;

async fn request_json(
    app: axum::Router,
    method: Method,
    path: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    request_json_with_auth(app, method, path, body, None).await
}

async fn request_json_with_auth(
    app: axum::Router,
    method: Method,
    path: &str,
    body: Option<Value>,
    bearer_token: Option<&str>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(path);
    if let Some(token) = bearer_token {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let request = if let Some(body) = body {
        builder = builder.header(header::CONTENT_TYPE, "application/json");
        builder.body(Body::from(body.to_string())).unwrap()
    } else {
        builder.body(Body::empty()).unwrap()
    };
    let response = app.oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let value = serde_json::from_slice(&bytes).unwrap();
    (status, value)
}

#[tokio::test]
async fn rest_help_returns_synapse_actions() {
    let app = server::router(loopback_state());
    let (status, body) = request_json(
        app,
        Method::POST,
        "/v1/synapse2",
        Some(json!({"action": "help", "params": {}})),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["tool"], "flux");
    assert!(body["actions"]["docker"].is_array());
}

#[tokio::test]
async fn rest_scout_nodes_works_without_auth_on_loopback_state() {
    let app = server::router(loopback_state());
    let (status, body) = request_json(
        app,
        Method::POST,
        "/v1/synapse2",
        Some(json!({"action": "scout.nodes", "params": {}})),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body["hosts"].is_array());
}

#[tokio::test]
async fn mounted_rest_read_scoped_dotted_actions_pass_scope_checks() {
    let app = server::router(synapse2::testing::bearer_state("read-token"));

    let (status, body) = request_json_with_auth(
        app.clone(),
        Method::POST,
        "/v1/synapse2",
        Some(json!({"action": "scout.nodes", "params": {}})),
        Some("read-token"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["hosts"].is_array());

    let (status, body) = request_json_with_auth(
        app,
        Method::POST,
        "/v1/synapse2",
        Some(json!({"action": "flux.docker.info", "params": {}})),
        Some("read-token"),
    )
    .await;
    assert_ne!(
        status,
        StatusCode::FORBIDDEN,
        "flux.docker.info should pass REST scope checks; body={body}"
    );
}

#[tokio::test]
async fn mounted_rest_dotted_write_actions_require_write_scope() {
    let app = server::router(synapse2::testing::bearer_state("read-token"));
    let (status, body) = request_json_with_auth(
        app,
        Method::POST,
        "/v1/synapse2",
        Some(json!({
            "action": "flux.docker.prune",
            "params": {
                "host": "local",
                "prune_target": "images",
                "force": true
            }
        })),
        Some("read-token"),
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert!(body["error"].as_str().unwrap().contains("synapse:write"));
}

#[tokio::test]
async fn rest_unknown_action_is_bad_request() {
    let app = server::router(loopback_state());
    let (status, body) = request_json(
        app,
        Method::POST,
        "/v1/synapse2",
        Some(json!({"action": "missing", "params": {}})),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(
        body["error"]
            .as_str()
            .unwrap()
            .contains("unknown synapse2 action")
    );
}

#[tokio::test]
async fn status_returns_only_local_redacted_metadata() {
    let app = server::router(loopback_state());
    let (status, body) = request_json(app, Method::GET, "/status", None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
    assert_eq!(body["server"], "synapse2");
    assert_eq!(body["transport"], "http");
    assert!(body.get("version").is_some());
    assert!(body.get("api_key").is_none(), "{body}");
}

#[tokio::test]
async fn activity_endpoint_reports_rest_calls_from_shared_state() {
    let app = server::router(loopback_state());
    let (status, _) = request_json(
        app.clone(),
        Method::POST,
        "/v1/synapse2",
        Some(json!({"action": "scout.nodes", "params": {}})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = request_json(app, Method::GET, "/activity", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["events"][0]["transport"], "rest");
    assert_eq!(body["events"][0]["action"], "scout.nodes");
    assert_eq!(body["events"][0]["ok"], true);
}

#[tokio::test]
async fn capabilities_report_authoritative_scopes() {
    let loopback = server::router(loopback_state());
    let (status, body) = request_json(loopback, Method::GET, "/capabilities", None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        body["scopes"]
            .as_array()
            .unwrap()
            .contains(&json!("synapse:read"))
    );
    assert!(
        body["scopes"]
            .as_array()
            .unwrap()
            .contains(&json!("synapse:write"))
    );

    let mounted = server::router(synapse2::testing::bearer_state("read-token"));
    let (status, _) = request_json(mounted.clone(), Method::GET, "/capabilities", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, body) = request_json_with_auth(
        mounted,
        Method::GET,
        "/capabilities",
        None,
        Some("read-token"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        body["scopes"]
            .as_array()
            .unwrap()
            .contains(&json!("synapse:read"))
    );
    assert!(
        !body["scopes"]
            .as_array()
            .unwrap()
            .contains(&json!("synapse:write"))
    );
    assert_eq!(body["destructive_allowed"], false);
}

#[tokio::test]
async fn readiness_is_separate_from_liveness() {
    let app = server::router(loopback_state());
    let (status, body) = request_json(app, Method::GET, "/ready", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ready");
}

/// Destructive REST actions (no elicitation channel) must return 403, not 500.
///
/// `flux.docker.prune` is write-scoped and confirmer-gated. On loopback state
/// (no auth), the request bypasses scope checks and reaches the `DenyConfirm`
/// gate which returns a typed `ConfirmationDenied` error. The REST handler must
/// map that to 403 Forbidden — not 500 — and not log at error level.
#[tokio::test]
async fn rest_destructive_action_confirmation_denied_returns_403_not_500() {
    // loopback_state has allow_destructive=false (default) and no auth,
    // so the request passes scope enforcement and reaches DenyConfirm.
    let app = server::router(loopback_state());
    let (status, body) = request_json(
        app,
        Method::POST,
        "/v1/synapse2",
        Some(json!({
            "action": "flux.docker.prune",
            "params": {
                "host": "local",
                "prune_target": "images",
                "force": true
            }
        })),
    )
    .await;

    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "flux.docker.prune with DenyConfirm must return 403 Forbidden, not 500; body={body}"
    );
    assert!(
        body["error"]
            .as_str()
            .unwrap_or_default()
            .contains("destructive"),
        "403 body must mention 'destructive'; body={body}"
    );
}

// Scout destructive ops must also map ConfirmationDenied → 403 (the typed
// error is preserved through the deadline wrapper rather than stringified).
#[tokio::test]
async fn rest_scout_exec_confirmation_denied_returns_403() {
    let app = server::router(loopback_state());
    let (status, body) = request_json(
        app,
        Method::POST,
        "/v1/synapse2",
        // `ls` is allowlisted, so it passes command validation and reaches the
        // DenyConfirm gate (scout exec is confirmation-gated).
        Some(json!({
            "action": "scout.exec",
            "params": { "host": "local", "path": "/tmp", "command": "ls" }
        })),
    )
    .await;

    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "scout.exec with DenyConfirm must return 403, not 500; body={body}"
    );
}

// Narrowness guard: a non-confirmation service error must NOT be mapped to 403
// (otherwise the 403 arm would mask genuine failures as policy denials).
#[tokio::test]
async fn rest_non_confirmation_error_is_not_403() {
    let app = server::router(loopback_state());
    let (status, body) = request_json(
        app,
        Method::POST,
        "/v1/synapse2",
        // `rm` is not allowlisted: command validation fails BEFORE the confirmer
        // gate, so this is not a ConfirmationDenied and must not return 403.
        Some(json!({
            "action": "scout.exec",
            "params": { "host": "local", "path": "/tmp", "command": "rm" }
        })),
    )
    .await;

    assert_ne!(
        status,
        StatusCode::FORBIDDEN,
        "a non-confirmation error must not be reported as 403; body={body}"
    );
}

#[tokio::test]
async fn oversized_body_returns_413() {
    let app = server::router(loopback_state());
    let oversized_body = vec![b'x'; 65_537];
    let request = Request::builder()
        .method(Method::POST)
        .uri("/v1/synapse2")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(oversized_body))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn readiness_redacts_internal_topology_errors() {
    struct FailingHosts;
    impl synapse2::host_config::HostRepository for FailingHosts {
        fn load_hosts(&self) -> anyhow::Result<Vec<synapse2::synapse::HostConfig>> {
            anyhow::bail!("secret path /private/topology.json failed")
        }
    }

    let mut state = loopback_state();
    state.service = state
        .service
        .with_host_repo(std::sync::Arc::new(FailingHosts));
    let (status, body) = request_json(server::router(state), Method::GET, "/ready", None).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["error"], "topology unavailable");
    assert!(!body.to_string().contains("/private/topology.json"));
}
