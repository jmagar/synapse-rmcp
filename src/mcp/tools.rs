//! MCP tool dispatch — thin shims only.
//!
//! **Rule**: no business logic here. Parse args → call service → return Value.
//! All logic belongs in `app.rs` (or `synapse2.rs` for transport concerns).
//!
//! The `peer` parameter is threaded through so that elicitation actions can
//! ask the MCP client for user input mid-call. For non-elicitation actions
//! it is unused.

use rmcp::{service::Peer, RoleServer};
use serde_json::Value;

use crate::actions::{execute_service_action, SynapseAction};
use crate::app::SynapseService;
use crate::server::AppState;

/// Dispatch an incoming MCP tool call to the appropriate handler.
///
/// `name`   — tool name (matches schema, currently only "synapse2")
/// `args`   — parsed JSON arguments from the MCP client
/// `peer`   — connection to the MCP client; used for elicitation
pub(super) async fn execute_tool(
    state: &AppState,
    name: &str,
    args: Value,
    peer: &Peer<RoleServer>,
) -> anyhow::Result<Value> {
    let _ = peer;
    match name {
        "flux" => dispatch_flux(state, args).await,
        "scout" => dispatch_scout(state, args).await,
        _ => Err(anyhow::anyhow!("unknown tool: {name}")),
    }
}

#[cfg(any(test, feature = "test-support"))]
#[doc(hidden)]
pub async fn execute_tool_without_peer_for_test(
    state: &AppState,
    name: &str,
    args: Value,
) -> anyhow::Result<Value> {
    match name {
        "flux" => dispatch_flux(state, args).await,
        "scout" => dispatch_scout(state, args).await,
        _ => Err(anyhow::anyhow!("unknown tool: {name}")),
    }
}

async fn dispatch_flux(state: &AppState, args: Value) -> anyhow::Result<Value> {
    let action = SynapseAction::from_flux_args(&args)?;
    dispatch_action(&state.service, &action).await
}

async fn dispatch_scout(state: &AppState, args: Value) -> anyhow::Result<Value> {
    let action = SynapseAction::from_scout_args(&args)?;
    dispatch_action(&state.service, &action).await
}

async fn dispatch_action(
    service: &SynapseService,
    action: &SynapseAction,
) -> anyhow::Result<Value> {
    execute_service_action(service, action).await
}

// ── arg helpers ───────────────────────────────────────────────────────────────

// ── help text ─────────────────────────────────────────────────────────────────

#[cfg(test)]
#[path = "tools_tests.rs"]
mod tests;
