//! REST API handlers — action dispatch, health/status/activity/capabilities, and OpenAPI.
//!
//! All handlers are thin: parse the request, call the service, return JSON.
//! Business logic lives in `app.rs`.

use anyhow::Result;
use axum::{
    extract::{Extension, State},
    http::{StatusCode, header},
    response::{IntoResponse, Json},
};
use lab_auth::AuthContext;
use serde::Deserialize;
use serde_json::{Value, json};
use std::time::Duration;

use crate::actions::{SynapseAction, execute_service_action};
use crate::server::{AppState, AuthPolicy};
use crate::token_limit::MAX_RESPONSE_BYTES;

/// Request body for `POST /v1/synapse2`.
///
/// REST uses an explicit `{ action, params }` envelope. MCP uses a flat
/// argument object such as `{ action, message }`. Both convert into the same
/// typed `SynapseAction` before calling `SynapseService`.
#[derive(Deserialize)]
pub struct ActionRequest {
    #[serde(default)]
    pub action: String,
    #[serde(default)]
    pub params: Value,
}

/// `POST /v1/synapse2` — dispatches an action by name.
///
/// Request:  `{"action": "flux.docker.info", "params": {}}`
pub async fn api_dispatch(
    State(state): State<AppState>,
    auth: Option<Extension<AuthContext>>,
    Json(body): Json<ActionRequest>,
) -> impl IntoResponse {
    let result =
        match crate::actions::rest::action_and_spec_from_request(&body.action, &body.params) {
            Ok((action, spec)) => {
                if let Some(response) = enforce_rest_scope(
                    &state,
                    auth.as_ref().map(|Extension(auth)| auth),
                    &action,
                    spec.required_scope,
                ) {
                    state
                        .activity
                        .record("rest", &body.action, false, Some("forbidden"));
                    return response;
                }
                // REST has no elicitation channel: destructive ops are hard-denied
                // (DenyConfirm) unless the SYNAPSE_MCP_ALLOW_DESTRUCTIVE override is
                // set, in which case NoConfirm runs them. Read-only ops are
                // unaffected (their service methods never call the confirmer).
                let confirmer: Box<dyn crate::elicitation_gate::Confirmer> =
                    if state.config.allow_destructive {
                        Box::new(crate::elicitation_gate::NoConfirm)
                    } else {
                        Box::new(crate::elicitation_gate::DenyConfirm)
                    };
                execute_service_action(&state.service, &action, confirmer.as_ref()).await
            }
            Err(error) => Err(error),
        };

    state.activity.record(
        "rest",
        &body.action,
        result.is_ok(),
        result
            .as_ref()
            .err()
            .map(|error| error.to_string())
            .as_deref(),
    );

    match result {
        Ok(value) => match cap_rest_response(value) {
            Ok(value) => Json(value).into_response(),
            Err(e) => {
                tracing::error!(error = %e, action = %body.action, "REST response serialization failed");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"error": "internal server error"})),
                )
                    .into_response()
            }
        },
        Err(e) if crate::actions::is_validation_error(&e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
        // Destructive-op confirmation denied (no elicitation channel over REST).
        // Return 403 Forbidden — not 500 — and do not log at error level.
        Err(e) if crate::actions::is_confirmation_denied(&e) => {
            tracing::debug!(action = %body.action, "REST destructive action denied: no confirmation channel");
            (
                StatusCode::FORBIDDEN,
                Json(json!({"error": "forbidden: destructive action requires confirmation; set SYNAPSE_MCP_ALLOW_DESTRUCTIVE=true or use MCP"})),
            )
                .into_response()
        }
        Err(e) => {
            tracing::error!(error = %e, action = %body.action, "REST action execution failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "internal server error"})),
            )
                .into_response()
        }
    }
}

/// REST compatibility subset of synapse2 actions.
///
/// This surface is intentionally limited to the dotted action names listed
/// below — it is NOT a full mirror of the MCP/CLI surface. New actions must be
/// explicitly added here; unknown dotted names fall through to the
/// `UnknownAction` error rather than accidentally routing to wrong subactions.
///
/// Parsing uses `split_once('.')` so that malformed names such as
/// `flux.docker.foo.bar` (three dots) are handled correctly: the first split
/// gives `("flux", "docker.foo.bar")` which is then matched against the exact
/// known second-segment set, and any non-matching value falls through to the
/// catch-all error.
fn cap_rest_response(value: Value) -> Result<Value> {
    let serialized = serde_json::to_vec(&value)?;
    if serialized.len() <= MAX_RESPONSE_BYTES {
        return Ok(value);
    }
    Ok(json!({
        "truncated": true,
        "error": "response exceeded REST response size limit",
        "max_response_bytes": MAX_RESPONSE_BYTES,
        "hint": "Use limit/offset parameters or more specific filters to get a smaller result.",
    }))
}

fn enforce_rest_scope(
    state: &AppState,
    auth: Option<&AuthContext>,
    action: &SynapseAction,
    required_scope: Option<&'static str>,
) -> Option<axum::response::Response> {
    if !matches!(&state.auth_policy, AuthPolicy::Mounted { .. }) {
        return None;
    }
    let required_scope = required_scope?;
    let Some(auth) = auth else {
        tracing::warn!(action = %action.name(), "REST action denied: missing auth context");
        return Some(
            (
                StatusCode::FORBIDDEN,
                Json(json!({"error": "forbidden: missing auth context"})),
            )
                .into_response(),
        );
    };
    let satisfied = crate::actions::scopes_satisfy(&auth.scopes, required_scope);
    if satisfied {
        return None;
    }
    tracing::warn!(
        subject = %auth.sub,
        action = %action.name(),
        required_scope = %required_scope,
        "REST action denied: insufficient scope"
    );
    Some(
        (
            StatusCode::FORBIDDEN,
            Json(json!({"error": format!("forbidden: requires scope: {required_scope}")})),
        )
            .into_response(),
    )
}

/// `GET /health` — liveness probe (unauthenticated).
pub async fn health() -> impl IntoResponse {
    tracing::debug!("health probe");
    Json(json!({ "status": "ok" }))
}

/// `GET /ready` — bounded readiness probe.
///
/// Unlike liveness, readiness verifies that the configured host topology can
/// be loaded. It deliberately does not dial every remote host: those checks
/// would make readiness slow and flap on an unrelated fleet member.
pub async fn ready(State(state): State<AppState>) -> impl IntoResponse {
    let service = state.service.clone();
    let check = tokio::task::spawn_blocking(move || service.scout().nodes_blocking());
    match tokio::time::timeout(Duration::from_secs(2), check).await {
        Ok(Ok(Ok(_))) => (StatusCode::OK, Json(json!({"status": "ready"}))).into_response(),
        Ok(Ok(Err(error))) => {
            tracing::warn!(error = %error, "readiness topology check failed");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({"status": "not_ready", "error": "topology unavailable"})),
            )
                .into_response()
        }
        Ok(Err(error)) => {
            tracing::warn!(error = %error, "readiness worker failed");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({"status": "not_ready", "error": "topology unavailable"})),
            )
                .into_response()
        }
        Err(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"status": "not_ready", "error": "topology check timed out"})),
        )
            .into_response(),
    }
}

/// `GET /openapi.json` — generated OpenAPI schema for the REST surface.
pub async fn openapi_json() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "application/json; charset=utf-8")],
        include_str!("../docs/generated/openapi.json"),
    )
}

/// `GET /status` — local runtime status (unauthenticated, redacts secrets).
pub async fn status(State(state): State<AppState>) -> impl IntoResponse {
    Json(status_payload(&state)).into_response()
}

pub(crate) fn status_payload(state: &AppState) -> Value {
    json!({
        "status": "ok",
        "server": state.config.server_name,
        "version": env!("CARGO_PKG_VERSION"),
        "transport": "http",
    })
}

/// `GET /activity` — bounded REST and MCP action history.
pub async fn activity(
    State(state): State<AppState>,
    auth: Option<Extension<AuthContext>>,
) -> impl IntoResponse {
    if matches!(&state.auth_policy, AuthPolicy::Mounted { .. }) {
        let Some(Extension(auth)) = auth else {
            return (StatusCode::FORBIDDEN, Json(json!({"error": "forbidden"}))).into_response();
        };
        if !crate::actions::scopes_satisfy(&auth.scopes, crate::actions::READ_SCOPE) {
            return (
                StatusCode::FORBIDDEN,
                Json(json!({"error": "forbidden: requires scope: synapse:read"})),
            )
                .into_response();
        }
    }
    Json(json!({"events": state.activity.snapshot()})).into_response()
}

/// `GET /capabilities` — authoritative authorization state for the web client.
pub async fn capabilities(
    State(state): State<AppState>,
    auth: Option<Extension<AuthContext>>,
) -> impl IntoResponse {
    let scopes = if matches!(&state.auth_policy, AuthPolicy::Mounted { .. }) {
        let Some(Extension(auth)) = auth else {
            return (StatusCode::FORBIDDEN, Json(json!({"error": "forbidden"}))).into_response();
        };
        auth.scopes
    } else {
        vec![
            crate::actions::READ_SCOPE.to_owned(),
            crate::actions::WRITE_SCOPE.to_owned(),
        ]
    };

    Json(json!({
        "scopes": scopes,
        "destructive_allowed": state.config.allow_destructive,
    }))
    .into_response()
}

#[cfg(test)]
#[path = "api_tests.rs"]
mod tests;
