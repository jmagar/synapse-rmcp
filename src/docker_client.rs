//! Docker transport layer for synapse2 (`flux` domain).
//!
//! This is the Docker spine: a [`DockerClient`] trait (segregated into
//! [`ContainerOps`], [`ImageOps`], [`NetworkOps`], [`VolumeOps`], [`SystemOps`]
//! sub-traits) plus the [`BollardClient`] implementation and a per-host
//! [`DockerClientCache`].
//!
//! Design (locked decisions — see bead rmcp-template-3tt.2):
//!
//! - **bollard everywhere.** No `docker` CLI subprocess for any operation. The
//!   client returns bollard's typed structs (`ContainerSummary`, `SystemInfo`,
//!   …) — the input to B4's formatters. No stdout parsing.
//! - **`bollard::Docker` is cheap to Clone** (internally `Arc<ClientType>` +
//!   `Arc<hyper>`), so each cache entry holds it **by value** inside a
//!   [`BollardClient`] bundle. For remote hosts the bundle also owns the
//!   [`ForwardedSocket`] + `Arc<PooledSession>` that keep the unix socket alive;
//!   bollard's `Docker` is only valid while that forward lives, so the bundle is
//!   the unit of caching (handed out as `Arc<BollardClient>`).
//! - **Per-host cache keyed by `HostConfig.name`.** One `BollardClient` per
//!   host, reused across calls. Concurrent creation for the same host is
//!   deduplicated through a per-key `OnceCell` — this also gives the
//!   "same instance on repeated lookup" property and prevents two racing callers
//!   from binding the *same* deterministic forward socket path.
//! - **Transport selection** mirrors synapse-mcp's `client-factory.ts`:
//!   - Local (`HostProtocol::Local` / `localhost`): an explicit
//!     `docker_socket_path` wins, else `connect_with_unix_defaults()`.
//!   - Remote (SSH): `pool.checkout(host)` → `ForwardedSocket::open(session,
//!     forward_socket_path(host))` → `connect_with_socket(path, …)`.
//! - **API version negotiation:** use bollard's `API_DEFAULT_VERSION` — do not
//!   hardcode a version string (locked decision).
//! - **BrokenPipe eviction** (HIGH, perf-oracle): a cached client whose SSH
//!   tunnel died returns IO `BrokenPipe` / `ConnectionRefused`. [`is_transport_dead`]
//!   classifies those; [`DockerClientCache::invalidate`] evicts the cache entry
//!   *and* the underlying SSH session so the next checkout rebuilds. B8/B9 do
//!   evict-then-retry.
//! - **Generous bollard timeout** (`CLIENT_TIMEOUT_SECS`, 120s): must exceed
//!   `SERVER_ALIVE_INTERVAL × count` so the SSH layer detects a dead connection
//!   first (perf-oracle).

use std::pin::Pin;
use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;
use bollard::container::LogOutput;
use bollard::exec::{CreateExecResults, StartExecOptions, StartExecResults};
use bollard::models::{
    ContainerInspectResponse, ContainerStatsResponse, ContainerSummary, ContainerTopResponse,
    ExecConfig, ExecInspectResponse, ImageSummary, Network, SystemDataUsageResponse, SystemInfo,
    VolumeListResponse,
};
use bollard::query_parameters::{
    DataUsageOptions, InspectContainerOptions, ListContainersOptions, ListImagesOptions,
    ListNetworksOptions, ListVolumesOptions, LogsOptions, StatsOptions, TopOptions,
};
use bollard::{Docker, API_DEFAULT_VERSION};
use dashmap::DashMap;
use futures_util::Stream;
use tokio::sync::OnceCell;

use crate::ssh::{forward_socket_path, ForwardedSocket, PooledSession, SshPool};
use crate::synapse::{HostConfig, HostProtocol};

#[cfg(test)]
#[path = "docker_client_tests.rs"]
mod tests;

/// bollard request timeout (seconds).
///
/// Deliberately generous: it must exceed `ssh::SERVER_ALIVE_INTERVAL` ×
/// ServerAliveCountMax so the SSH layer detects a dead control connection before
/// bollard times out the HTTP request (perf-oracle, see module docs).
pub const CLIENT_TIMEOUT_SECS: u64 = 120;

/// A boxed, `Send` stream — the return type for the streaming Docker surfaces
/// (`logs`, `stats`, attached exec output). `dyn DockerClient` requires a
/// concrete (boxed) return type rather than `impl Stream`.
pub type BoxStream<T> = Pin<Box<dyn Stream<Item = Result<T, bollard::errors::Error>> + Send>>;

// ---------------------------------------------------------------------------
// Segregated operation traits (ISP) — composed into `DockerClient`.
// Mocks implement only the sub-traits they exercise.
// ---------------------------------------------------------------------------

/// Container lifecycle, inspection, and streaming operations.
///
/// Read ops (`list`, `inspect`, `top`) are awaited single calls. `logs`/`stats`
/// return **streams** (bollard 0.21) — consumers (B8) drive them, applying their
/// own bounded backpressure. Exec is the 3-step `create_exec` → `start_exec`
/// (stream) → `inspect_exec` flow (B9); this trait exposes each primitive.
#[async_trait]
pub trait ContainerOps: Send + Sync {
    async fn list_containers(
        &self,
        options: Option<ListContainersOptions>,
    ) -> Result<Vec<ContainerSummary>, bollard::errors::Error>;

    async fn inspect_container(
        &self,
        name: &str,
        options: Option<InspectContainerOptions>,
    ) -> Result<ContainerInspectResponse, bollard::errors::Error>;

    async fn top_processes(
        &self,
        name: &str,
        options: Option<TopOptions>,
    ) -> Result<ContainerTopResponse, bollard::errors::Error>;

    /// Container logs as a stream. Unbounded at the source — B8 applies a
    /// bounded mpsc buffer for backpressure.
    fn logs(&self, name: &str, options: Option<LogsOptions>) -> BoxStream<LogOutput>;

    /// Live resource stats as a stream.
    fn stats(&self, name: &str, options: Option<StatsOptions>)
        -> BoxStream<ContainerStatsResponse>;

    /// Lifecycle action by container `name` (start/stop/restart/pause/unpause/
    /// kill/remove). `action` is the bollard endpoint verb; B9 maps user actions
    /// to these. Implemented as a thin passthrough.
    async fn container_action(
        &self,
        name: &str,
        action: ContainerAction,
    ) -> Result<(), bollard::errors::Error>;

    // --- exec, 3-step (B9) ---
    async fn create_exec(
        &self,
        name: &str,
        config: ExecConfig,
    ) -> Result<CreateExecResults, bollard::errors::Error>;

    async fn start_exec(
        &self,
        exec_id: &str,
        options: Option<StartExecOptions>,
    ) -> Result<StartExecResults, bollard::errors::Error>;

    async fn inspect_exec(
        &self,
        exec_id: &str,
    ) -> Result<ExecInspectResponse, bollard::errors::Error>;
}

/// Lifecycle verbs for [`ContainerOps::container_action`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerAction {
    Start,
    Stop,
    Restart,
    Pause,
    Unpause,
    Kill,
    Remove,
}

/// Image management and inspection.
#[async_trait]
pub trait ImageOps: Send + Sync {
    async fn list_images(
        &self,
        options: Option<ListImagesOptions>,
    ) -> Result<Vec<ImageSummary>, bollard::errors::Error>;
}

/// Network resource operations.
#[async_trait]
pub trait NetworkOps: Send + Sync {
    async fn list_networks(
        &self,
        options: Option<ListNetworksOptions>,
    ) -> Result<Vec<Network>, bollard::errors::Error>;
}

/// Volume resource operations.
#[async_trait]
pub trait VolumeOps: Send + Sync {
    async fn list_volumes(
        &self,
        options: Option<ListVolumesOptions>,
    ) -> Result<VolumeListResponse, bollard::errors::Error>;
}

/// System-level information and health.
#[async_trait]
pub trait SystemOps: Send + Sync {
    async fn info(&self) -> Result<SystemInfo, bollard::errors::Error>;

    async fn df(
        &self,
        options: Option<DataUsageOptions>,
    ) -> Result<SystemDataUsageResponse, bollard::errors::Error>;

    /// Liveness probe. Used by [`DockerClientCache`] for explicit health checks
    /// and by B8/B9 to detect a dead tunnel.
    async fn ping(&self) -> Result<String, bollard::errors::Error>;
}

/// The composed Docker client surface that action beads (B8–B13) depend on.
///
/// Object-safe (`Send + Sync`, boxed streams) so it can be used as
/// `&dyn DockerClient` in free-function action seams (mirroring scout's
/// `repo: &dyn HostRepository` pattern) and mocked without live docker.
pub trait DockerClient: ContainerOps + ImageOps + NetworkOps + VolumeOps + SystemOps {}

// Blanket impl: anything satisfying every sub-trait is a `DockerClient`.
impl<T> DockerClient for T where T: ContainerOps + ImageOps + NetworkOps + VolumeOps + SystemOps {}

/// Classify a `bollard::errors::Error` as a dead-transport condition that should
/// trigger cache eviction + rebuild (HIGH, perf-oracle).
///
/// Matches IO `BrokenPipe` / `ConnectionRefused` / `ConnectionReset` plus
/// hyper/HTTP-client failures (the surface a severed SSH-forwarded socket
/// produces).
pub fn is_transport_dead(err: &bollard::errors::Error) -> bool {
    use bollard::errors::Error as E;
    use std::io::ErrorKind;
    match err {
        E::IOError { err } => matches!(
            err.kind(),
            ErrorKind::BrokenPipe
                | ErrorKind::ConnectionRefused
                | ErrorKind::ConnectionReset
                | ErrorKind::ConnectionAborted
                | ErrorKind::NotConnected
                | ErrorKind::UnexpectedEof
        ),
        // hyper / lower-level HTTP client errors over a severed socket.
        E::HyperResponseError { .. } | E::HttpClientError { .. } | E::HyperLegacyError { .. } => {
            true
        }
        E::RequestTimeoutError => true,
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// BollardClient — the real implementation + its owned transport guard.
// ---------------------------------------------------------------------------

/// A cached, live Docker client for a single host.
///
/// For a **remote** host the bundle owns the [`ForwardedSocket`] guard and the
/// `Arc<PooledSession>` that keep the unix socket alive — bollard's `Docker` is
/// only valid while they live, so they are dropped together when the cache entry
/// is evicted. For a **local** host both are `None`.
pub struct BollardClient {
    docker: Docker,
    /// Held for the client's lifetime (remote only). On drop, the forward is
    /// torn down. Prefer explicit teardown via [`BollardClient::close`].
    forward: Option<ForwardedSocket>,
    /// Keeps the SSH session alive for the duration of the forward (remote only).
    _session: Option<Arc<PooledSession>>,
}

impl BollardClient {
    /// Connect to a **local** docker daemon. An explicit `docker_socket_path`
    /// wins (mirrors the TS factory's socket-path-first check); otherwise
    /// bollard's unix defaults (`DOCKER_HOST` / `/var/run/docker.sock`).
    pub fn connect_local(host: &HostConfig) -> Result<Self> {
        let docker = match host.docker_socket_path.as_deref() {
            Some(path) => {
                Docker::connect_with_socket(path, CLIENT_TIMEOUT_SECS, API_DEFAULT_VERSION)
                    .with_context(|| {
                        format!(
                            "connect bollard to local socket {path} for host {}",
                            host.name
                        )
                    })?
            }
            None => Docker::connect_with_unix_defaults().with_context(|| {
                format!("connect bollard to local docker for host {}", host.name)
            })?,
        };
        Ok(Self {
            docker,
            forward: None,
            _session: None,
        })
    }

    /// Connect to a **remote** docker daemon via a B1 SSH-forwarded unix socket.
    ///
    /// Checks out the shared SSH session, opens a 0600 forward to the remote
    /// `/var/run/docker.sock`, and points bollard at the local socket path. The
    /// forward + session are held inside the returned bundle for its lifetime.
    pub async fn connect_remote(pool: &SshPool, host: &HostConfig) -> Result<Self> {
        let pooled = pool.checkout(host).await?;
        let session = pooled.session();
        let forward = ForwardedSocket::open(session, forward_socket_path(host))
            .await
            .with_context(|| format!("forward docker socket for host {}", host.name))?;

        let path = forward.path().to_string_lossy().into_owned();
        let docker = Docker::connect_with_socket(&path, CLIENT_TIMEOUT_SECS, API_DEFAULT_VERSION)
            .with_context(|| {
            format!(
                "connect bollard to forwarded socket {path} for host {}",
                host.name
            )
        })?;

        Ok(Self {
            docker,
            forward: Some(forward),
            _session: Some(pooled),
        })
    }

    /// Borrow the underlying bollard `Docker` (cheap to clone if a caller needs
    /// an owned handle for streaming).
    pub fn docker(&self) -> &Docker {
        &self.docker
    }

    /// Explicit async teardown of the forwarded socket (remote only). Preferred
    /// over relying on `Drop` so the port-forward is closed deterministically.
    pub async fn close(self) -> Result<()> {
        if let Some(forward) = self.forward {
            forward.close().await?;
        }
        Ok(())
    }
}

#[async_trait]
impl ContainerOps for BollardClient {
    async fn list_containers(
        &self,
        options: Option<ListContainersOptions>,
    ) -> Result<Vec<ContainerSummary>, bollard::errors::Error> {
        self.docker.list_containers(options).await
    }

    async fn inspect_container(
        &self,
        name: &str,
        options: Option<InspectContainerOptions>,
    ) -> Result<ContainerInspectResponse, bollard::errors::Error> {
        self.docker.inspect_container(name, options).await
    }

    async fn top_processes(
        &self,
        name: &str,
        options: Option<TopOptions>,
    ) -> Result<ContainerTopResponse, bollard::errors::Error> {
        self.docker.top_processes(name, options).await
    }

    fn logs(&self, name: &str, options: Option<LogsOptions>) -> BoxStream<LogOutput> {
        Box::pin(self.docker.logs(name, options))
    }

    fn stats(
        &self,
        name: &str,
        options: Option<StatsOptions>,
    ) -> BoxStream<ContainerStatsResponse> {
        Box::pin(self.docker.stats(name, options))
    }

    async fn container_action(
        &self,
        name: &str,
        action: ContainerAction,
    ) -> Result<(), bollard::errors::Error> {
        use bollard::query_parameters as q;
        match action {
            ContainerAction::Start => {
                self.docker
                    .start_container(name, None::<q::StartContainerOptions>)
                    .await
            }
            ContainerAction::Stop => {
                self.docker
                    .stop_container(name, None::<q::StopContainerOptions>)
                    .await
            }
            ContainerAction::Restart => {
                self.docker
                    .restart_container(name, None::<q::RestartContainerOptions>)
                    .await
            }
            ContainerAction::Pause => self.docker.pause_container(name).await,
            ContainerAction::Unpause => self.docker.unpause_container(name).await,
            ContainerAction::Kill => {
                self.docker
                    .kill_container(name, None::<q::KillContainerOptions>)
                    .await
            }
            ContainerAction::Remove => {
                self.docker
                    .remove_container(name, None::<q::RemoveContainerOptions>)
                    .await
            }
        }
    }

    async fn create_exec(
        &self,
        name: &str,
        config: ExecConfig,
    ) -> Result<CreateExecResults, bollard::errors::Error> {
        self.docker.create_exec(name, config).await
    }

    async fn start_exec(
        &self,
        exec_id: &str,
        options: Option<StartExecOptions>,
    ) -> Result<StartExecResults, bollard::errors::Error> {
        self.docker.start_exec(exec_id, options).await
    }

    async fn inspect_exec(
        &self,
        exec_id: &str,
    ) -> Result<ExecInspectResponse, bollard::errors::Error> {
        self.docker.inspect_exec(exec_id).await
    }
}

#[async_trait]
impl ImageOps for BollardClient {
    async fn list_images(
        &self,
        options: Option<ListImagesOptions>,
    ) -> Result<Vec<ImageSummary>, bollard::errors::Error> {
        self.docker.list_images(options).await
    }
}

#[async_trait]
impl NetworkOps for BollardClient {
    async fn list_networks(
        &self,
        options: Option<ListNetworksOptions>,
    ) -> Result<Vec<Network>, bollard::errors::Error> {
        self.docker.list_networks(options).await
    }
}

#[async_trait]
impl VolumeOps for BollardClient {
    async fn list_volumes(
        &self,
        options: Option<ListVolumesOptions>,
    ) -> Result<VolumeListResponse, bollard::errors::Error> {
        self.docker.list_volumes(options).await
    }
}

#[async_trait]
impl SystemOps for BollardClient {
    async fn info(&self) -> Result<SystemInfo, bollard::errors::Error> {
        self.docker.info().await
    }

    async fn df(
        &self,
        options: Option<DataUsageOptions>,
    ) -> Result<SystemDataUsageResponse, bollard::errors::Error> {
        self.docker.df(options).await
    }

    async fn ping(&self) -> Result<String, bollard::errors::Error> {
        self.docker.ping().await
    }
}

// ---------------------------------------------------------------------------
// DockerClientCache — per-host, dedup via OnceCell, BrokenPipe eviction.
// ---------------------------------------------------------------------------

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
    fn is_local(host: &HostConfig) -> bool {
        host.protocol == HostProtocol::Local || host.host == "localhost"
    }

    /// Evict a host's cached client **and** its SSH session.
    ///
    /// Called on a dead-transport error ([`is_transport_dead`]) so the next
    /// [`client_for`](Self::client_for) rebuilds against a fresh tunnel
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

// ---------------------------------------------------------------------------
// MockDockerClient — a hand-written test double for the trait surface.
//
// Lives behind `test-support` (not bare `cfg(test)`) so the integration-test
// crate (B8/B9/B10/B13 in `tests/`) can reuse it — `tests/` is a separate crate
// and cannot see `#[cfg(test)]`-only items. Re-exported via `lib::testing`.
// ---------------------------------------------------------------------------

/// A scriptable in-memory [`DockerClient`] for tests. Each field holds the
/// canned response for the corresponding operation; streaming/exec surfaces
/// default to empty/error so consumers can override only what they exercise.
#[cfg(any(test, feature = "test-support"))]
#[derive(Default)]
pub struct MockDockerClient {
    pub containers: Vec<ContainerSummary>,
    pub images: Vec<ImageSummary>,
    pub networks: Vec<Network>,
    pub volumes: VolumeListResponse,
    pub info: SystemInfo,
    pub df: SystemDataUsageResponse,
    pub ping: String,
    /// Optional canned container inspection keyed by name.
    pub inspect: std::collections::HashMap<String, ContainerInspectResponse>,
    /// Optional canned top output keyed by name.
    pub top: std::collections::HashMap<String, ContainerTopResponse>,
    /// Records every lifecycle action requested, for assertions.
    pub actions: std::sync::Mutex<Vec<(String, ContainerAction)>>,
}

#[cfg(any(test, feature = "test-support"))]
impl MockDockerClient {
    pub fn new() -> Self {
        Self::default()
    }

    /// Recorded `(name, action)` lifecycle calls.
    pub fn recorded_actions(&self) -> Vec<(String, ContainerAction)> {
        self.actions.lock().expect("mock action log").clone()
    }
}

#[cfg(any(test, feature = "test-support"))]
#[async_trait]
impl ContainerOps for MockDockerClient {
    async fn list_containers(
        &self,
        _options: Option<ListContainersOptions>,
    ) -> Result<Vec<ContainerSummary>, bollard::errors::Error> {
        Ok(self.containers.clone())
    }

    async fn inspect_container(
        &self,
        name: &str,
        _options: Option<InspectContainerOptions>,
    ) -> Result<ContainerInspectResponse, bollard::errors::Error> {
        Ok(self.inspect.get(name).cloned().unwrap_or_default())
    }

    async fn top_processes(
        &self,
        name: &str,
        _options: Option<TopOptions>,
    ) -> Result<ContainerTopResponse, bollard::errors::Error> {
        Ok(self.top.get(name).cloned().unwrap_or_default())
    }

    fn logs(&self, _name: &str, _options: Option<LogsOptions>) -> BoxStream<LogOutput> {
        Box::pin(futures_util::stream::empty())
    }

    fn stats(
        &self,
        _name: &str,
        _options: Option<StatsOptions>,
    ) -> BoxStream<ContainerStatsResponse> {
        Box::pin(futures_util::stream::empty())
    }

    async fn container_action(
        &self,
        name: &str,
        action: ContainerAction,
    ) -> Result<(), bollard::errors::Error> {
        self.actions
            .lock()
            .expect("mock action log")
            .push((name.to_string(), action));
        Ok(())
    }

    async fn create_exec(
        &self,
        _name: &str,
        _config: ExecConfig,
    ) -> Result<CreateExecResults, bollard::errors::Error> {
        Ok(CreateExecResults {
            id: "mock-exec".to_string(),
        })
    }

    async fn start_exec(
        &self,
        _exec_id: &str,
        _options: Option<StartExecOptions>,
    ) -> Result<StartExecResults, bollard::errors::Error> {
        Ok(StartExecResults::Detached)
    }

    async fn inspect_exec(
        &self,
        _exec_id: &str,
    ) -> Result<ExecInspectResponse, bollard::errors::Error> {
        Ok(ExecInspectResponse::default())
    }
}

#[cfg(any(test, feature = "test-support"))]
#[async_trait]
impl ImageOps for MockDockerClient {
    async fn list_images(
        &self,
        _options: Option<ListImagesOptions>,
    ) -> Result<Vec<ImageSummary>, bollard::errors::Error> {
        Ok(self.images.clone())
    }
}

#[cfg(any(test, feature = "test-support"))]
#[async_trait]
impl NetworkOps for MockDockerClient {
    async fn list_networks(
        &self,
        _options: Option<ListNetworksOptions>,
    ) -> Result<Vec<Network>, bollard::errors::Error> {
        Ok(self.networks.clone())
    }
}

#[cfg(any(test, feature = "test-support"))]
#[async_trait]
impl VolumeOps for MockDockerClient {
    async fn list_volumes(
        &self,
        _options: Option<ListVolumesOptions>,
    ) -> Result<VolumeListResponse, bollard::errors::Error> {
        Ok(self.volumes.clone())
    }
}

#[cfg(any(test, feature = "test-support"))]
#[async_trait]
impl SystemOps for MockDockerClient {
    async fn info(&self) -> Result<SystemInfo, bollard::errors::Error> {
        Ok(self.info.clone())
    }

    async fn df(
        &self,
        _options: Option<DataUsageOptions>,
    ) -> Result<SystemDataUsageResponse, bollard::errors::Error> {
        Ok(self.df.clone())
    }

    async fn ping(&self) -> Result<String, bollard::errors::Error> {
        Ok(self.ping.clone())
    }
}
