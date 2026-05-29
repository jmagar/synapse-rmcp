//! Per-host Docker client cache with SSH-pool integration and BrokenPipe eviction.
//!
//! [`DockerClientCache`] holds one [`BollardClient`] per `HostConfig.name`,
//! reused across calls. Concurrent creation for the same host is deduplicated
//! through a per-key [`OnceCell`].

use std::sync::Arc;

use anyhow::Result;
use dashmap::DashMap;
use tokio::sync::OnceCell;

use crate::ssh::SshPool;
use crate::synapse::{HostConfig, HostProtocol};

use super::bollard_client::BollardClient;

/// Per-host Docker client cache. One [`BollardClient`] per `HostConfig.name`,
/// reused across calls. Owns the [`SshPool`] used to forward remote sockets.
///
/// Concurrent creation for the same host is deduplicated through a per-key
/// [`OnceCell`] — racing callers await the same init, which both prevents
/// duplicate connections and avoids two callers binding the same deterministic
/// forward socket path.
pub struct DockerClientCache {
    pool: Arc<SshPool>,
    clients: DashMap<String, Arc<OnceCell<Arc<BollardClient>>>>,
}

impl DockerClientCache {
    pub fn new() -> Self {
        Self::with_pool(Arc::new(SshPool::new()))
    }

    /// Use an externally-owned SSH pool (e.g. shared with `scout` remote exec).
    pub fn with_pool(pool: Arc<SshPool>) -> Self {
        Self {
            pool,
            clients: DashMap::new(),
        }
    }

    /// The SSH pool backing remote forwards (shared with other consumers).
    pub fn pool(&self) -> &Arc<SshPool> {
        &self.pool
    }

    /// Get (or build) the cached client for `host`. Two consecutive calls for
    /// the same host name return the **same** `Arc<BollardClient>`.
    ///
    /// Never holds a `DashMap` guard across `.await`: the per-key `OnceCell` is
    /// cloned out (cheap `Arc`) before the (possibly slow) init runs.
    pub async fn client_for(&self, host: &HostConfig) -> Result<Arc<BollardClient>> {
        let cell = self
            .clients
            .entry(host.name.clone())
            .or_insert_with(|| Arc::new(OnceCell::new()))
            .clone();

        cell.get_or_try_init(|| async {
            let client = if Self::is_local(host) {
                BollardClient::connect_local(host)?
            } else {
                BollardClient::connect_remote(&self.pool, host).await?
            };
            Ok::<_, anyhow::Error>(Arc::new(client))
        })
        .await
        .cloned()
    }

    /// Is this host served by the local docker daemon (no SSH forward needed)?
    pub(crate) fn is_local(host: &HostConfig) -> bool {
        host.protocol == HostProtocol::Local || host.host == "localhost"
    }

    /// Evict a host's cached client **and** its SSH session.
    ///
    /// Called on a dead-transport error ([`super::is_transport_dead`]) so the
    /// next [`client_for`](Self::client_for) rebuilds against a fresh tunnel
    /// (HIGH, perf-oracle). Dropping the `BollardClient` tears down its forward.
    pub fn invalidate(&self, host: &HostConfig) {
        self.clients.remove(&host.name);
        self.pool.invalidate(host);
    }

    /// Number of cached client entries (observability / test assertions).
    /// Counts only entries whose `OnceCell` has been initialized.
    pub fn len(&self) -> usize {
        self.clients
            .iter()
            .filter(|e| e.value().initialized())
            .count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Drop every cached client (forces fresh connections; used on shutdown).
    pub fn clear(&self) {
        self.clients.clear();
    }
}

impl Default for DockerClientCache {
    fn default() -> Self {
        Self::new()
    }
}
