//! Flux domain service — Docker / container / host / compose operations.
//!
//! Extracted from the `SynapseService` god-object so flux concerns live in one
//! focused module. Owns the per-host bollard Docker client cache (B2) and the
//! compose discovery engine + TTL cache (B12). Resolves hosts through the
//! injected `HostRepository`.
//!
//! All flux business logic lives here. CLI (`cli.rs`) and MCP (via `actions.rs`)
//! are thin shims that call into these methods.

use anyhow::Result;
use serde_json::{json, Value};
use std::sync::Arc;

use crate::compose::{ComposeDiscovery, ComposeProject};
use crate::docker;
use crate::docker_client::DockerClientCache;
use crate::host_config::HostRepository;
use crate::scout;

#[cfg(test)]
#[path = "flux_service_tests.rs"]
mod tests;

/// Flux domain service. Cheap to clone — all fields are `Arc`-shared.
#[derive(Clone)]
pub struct FluxService {
    /// Host configuration repository — shared with the facade and scout so an
    /// injected repo (tests / DI) resolves the same hosts everywhere.
    pub(crate) host_repo: Arc<dyn HostRepository>,
    /// Compose project discovery engine + per-host TTL cache (B12). Held behind
    /// `Arc` so the shared cache survives clones.
    pub(crate) compose: Arc<ComposeDiscovery>,
    /// Per-host bollard Docker client cache (B2). One client per `HostConfig`,
    /// reused; remote hosts connect via B1's SSH-forwarded unix socket.
    pub(crate) docker_clients: Arc<DockerClientCache>,
}

impl FluxService {
    /// Construct with the supplied host repository and default discovery / client caches.
    pub fn new(host_repo: Arc<dyn HostRepository>) -> Self {
        Self {
            host_repo,
            compose: Arc::new(ComposeDiscovery::new(Arc::new(crate::ssh::SshPool::new()))),
            docker_clients: Arc::new(DockerClientCache::new()),
        }
    }

    pub async fn help(&self) -> Result<Value> {
        Ok(json!({
            "tool": "flux",
            "actions": {
                "docker": ["info", "images", "networks", "volumes"],
                "container": ["list", "inspect", "logs"],
                "host": ["status"],
                "help": []
            },
            "deferred": ["compose", "destructive container lifecycle", "docker prune/rmi"],
        }))
    }

    pub async fn docker_info(&self) -> Result<Value> {
        docker::docker_json(&["info", "--format", "{{json .}}"]).await
    }

    pub async fn docker_images(&self) -> Result<Value> {
        docker::docker_json(&["images", "--format", "{{json .}}"]).await
    }

    pub async fn docker_networks(&self) -> Result<Value> {
        docker::docker_json(&["network", "ls", "--format", "{{json .}}"]).await
    }

    pub async fn docker_volumes(&self) -> Result<Value> {
        docker::docker_json(&["volume", "ls", "--format", "{{json .}}"]).await
    }

    pub async fn container_list(&self) -> Result<Value> {
        docker::docker_json(&["container", "ls", "-a", "--format", "{{json .}}"]).await
    }

    pub async fn container_inspect(&self, container_id: &str) -> Result<Value> {
        docker::docker_json(&["container", "inspect", container_id]).await
    }

    pub async fn container_logs(&self, container_id: &str, lines: u32) -> Result<Value> {
        let lines = lines.clamp(1, 500).to_string();
        docker::docker_json(&["container", "logs", "--tail", &lines, container_id]).await
    }

    pub async fn host_status(&self, host: Option<&str>) -> Result<Value> {
        Ok(json!({
            "host": host.unwrap_or("local"),
            "docker": self.docker_info().await?,
        }))
    }

    /// Discover compose projects on `host_name`, merging `docker compose ls`
    /// with a filesystem scan (cache-aware). Thin delegation to the discovery
    /// engine; resolves the host via the injected repository.
    pub async fn compose_list(&self, host_name: &str) -> Result<Vec<ComposeProject>> {
        let host = scout::resolve_host(self.host_repo.as_ref(), host_name)?;
        self.compose.list(&host).await
    }

    /// Invalidate the compose discovery cache for `host_name` (or all hosts when
    /// `None`), forcing the next `compose_list` to re-scan.
    pub fn compose_refresh(&self, host_name: Option<&str>) {
        self.compose.refresh(host_name);
    }
}
