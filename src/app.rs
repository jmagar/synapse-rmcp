//! Business service layer — thin facade over focused domain services.
//!
//! **All business logic lives in the domain services.** CLI and MCP are thin
//! shims that call into this facade, which delegates to the sub-services.
//!
//! `SynapseService` owns:
//! - `flux: FluxService` — Docker / container / host / compose operations
//! - `scout: ScoutService` — node discovery, filesystem peek, remote exec
//! - `client: SynapseClient` — template transport (greet/echo/status)
//!
//! Reach domain methods through the accessors: `service.flux().docker_info()`,
//! `service.scout().nodes()`. If you need caching, retries, data transformation,
//! or validation, do it in the relevant domain service — never in `cli.rs` or
//! `mcp/tools.rs`.

use anyhow::Result;
use serde_json::{json, Value};
use std::sync::Arc;

use crate::compose::ComposeDiscovery;
use crate::docker_client::DockerClientCache;
use crate::flux_service::FluxService;
use crate::host_config::{FileHostRepository, HostRepository};
use crate::scout_service::ScoutService;
use crate::synapse2::SynapseClient;

// Re-export the scaffold contract types so existing callers that import them
// from `crate::app` (e.g. actions.rs's downcast, app_tests.rs) keep compiling.
pub use crate::scaffold::{ScaffoldIntent, ScaffoldIntentValidationError};

// Unit tests live in a sidecar file — see src/app_tests.rs for the pattern.
#[cfg(test)]
#[path = "app_tests.rs"]
mod tests;

/// The service layer — a thin facade wiring together the focused domain
/// services plus the template transport client.
#[derive(Clone)]
pub struct SynapseService {
    client: SynapseClient,
    flux: FluxService,
    scout: ScoutService,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElicitedNameOutcome<'a> {
    Accepted(&'a str),
    NoInput,
    Declined,
    Cancelled,
    Unsupported,
}

impl SynapseService {
    /// Create a new `SynapseService` with the production host repository.
    ///
    /// The `client` parameter signature is unchanged — existing callers compile as-is.
    pub fn new(client: SynapseClient) -> Self {
        let host_repo: Arc<dyn HostRepository> = Arc::new(FileHostRepository::default());
        Self {
            client,
            flux: FluxService::new(Arc::clone(&host_repo)),
            scout: ScoutService::new(host_repo),
        }
    }

    /// Inject a custom `HostRepository` (for testing or future DI).
    ///
    /// Propagates to **both** the flux and scout sub-services so they resolve
    /// the same hosts.
    pub fn with_host_repo(mut self, repo: Arc<dyn HostRepository>) -> Self {
        self.flux.host_repo = Arc::clone(&repo);
        self.scout.host_repo = repo;
        self
    }

    /// Inject a custom compose discovery engine (for testing or future DI).
    pub fn with_compose_discovery(mut self, compose: Arc<ComposeDiscovery>) -> Self {
        self.flux.compose = compose;
        self
    }

    /// Inject a custom `DockerClientCache` (e.g. one sharing an `SshPool` with
    /// scout, or a cache primed for tests).
    pub fn with_docker_clients(mut self, cache: Arc<DockerClientCache>) -> Self {
        self.flux.docker_clients = cache;
        self
    }

    /// Access the flux domain service (Docker / container / host / compose).
    pub fn flux(&self) -> &FluxService {
        &self.flux
    }

    /// Access the scout domain service (nodes / peek / exec).
    pub fn scout(&self) -> &ScoutService {
        &self.scout
    }

    /// Return a greeting for `name`, defaulting to "World".
    pub async fn greet(&self, name: Option<&str>) -> Result<Value> {
        self.client.greet(name).await
    }

    /// Echo `message` back unchanged.
    pub async fn echo(&self, message: &str) -> Result<Value> {
        self.client.echo(message).await
    }

    /// Return the server status.
    pub async fn status(&self) -> Result<Value> {
        self.client.status().await
    }

    /// Build the response for the elicited-name demo after the MCP shim collects input.
    pub fn elicited_name_greeting(&self, outcome: ElicitedNameOutcome<'_>) -> Value {
        match outcome {
            ElicitedNameOutcome::Accepted(name) => {
                let name = name.trim().to_owned();
                if name.is_empty() {
                    json!({
                        "greeting": "Hello, mysterious stranger!",
                        "note": "You submitted an empty name - that's perfectly fine!",
                    })
                } else {
                    json!({
                        "greeting": format!("Hello, {name}! Welcome to the synapse2 MCP server."),
                        "name": name,
                    })
                }
            }
            ElicitedNameOutcome::NoInput => json!({
                "greeting": "Hello! (you provided no name - that's okay)",
            }),
            ElicitedNameOutcome::Declined => json!({
                "message": "No problem - you chose not to share your name.",
                "greeting": "Hello, anonymous user!",
            }),
            ElicitedNameOutcome::Cancelled => json!({
                "message": "Elicitation was cancelled.",
                "greeting": "Hello there!",
            }),
            ElicitedNameOutcome::Unsupported => json!({
                "message": "Elicitation is not supported by this MCP client.",
                "hint": "Try a client like Claude.app that supports MCP elicitation (spec 2025-06-18).",
                "fallback_greeting": "Hello, World! (elicitation unavailable)",
            }),
        }
    }

    /// Convert elicited scaffold requirements into the handoff contract consumed
    /// by the skill. Thin delegation to the `scaffold` module.
    pub fn scaffold_intent(&self, input: ScaffoldIntent) -> Result<Value> {
        crate::scaffold::scaffold_intent(input)
    }
}
