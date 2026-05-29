//! Scout domain service — node discovery, filesystem peek, remote exec.
//!
//! Extracted from the `SynapseService` god-object so scout concerns live in one
//! focused module. Resolves hosts through the injected `HostRepository`.
//!
//! All scout business logic lives here. CLI (`cli.rs`) and MCP (via `actions.rs`)
//! are thin shims that call into these methods.

use anyhow::Result;
use serde_json::{json, Value};
use std::sync::Arc;

use crate::host_config::HostRepository;
use crate::scout;

#[cfg(test)]
#[path = "scout_service_tests.rs"]
mod tests;

/// Scout domain service. Cheap to clone — the repository is `Arc`-shared.
#[derive(Clone)]
pub struct ScoutService {
    /// Host configuration repository — shared with the facade and flux so an
    /// injected repo (tests / DI) resolves the same hosts everywhere.
    pub(crate) host_repo: Arc<dyn HostRepository>,
}

impl ScoutService {
    /// Construct with the supplied host repository.
    pub fn new(host_repo: Arc<dyn HostRepository>) -> Self {
        Self { host_repo }
    }

    pub async fn help(&self) -> Result<Value> {
        Ok(json!({
            "tool": "scout",
            "actions": ["nodes", "peek", "exec", "help"],
            "deferred": ["find", "delta", "emit", "beam", "ps", "df", "zfs", "logs"],
        }))
    }

    pub async fn nodes(&self) -> Result<Value> {
        scout::nodes(self.host_repo.as_ref())
    }

    pub async fn peek(&self, host: &str, path: &str) -> Result<Value> {
        scout::peek(self.host_repo.as_ref(), host, path)
    }

    pub async fn exec(&self, host: &str, path: &str, command: &str) -> Result<Value> {
        scout::exec(self.host_repo.as_ref(), host, path, command)
    }
}
