//! Segregated operation traits (ISP) and the composed [`DockerClient`] super-trait.
//!
//! Each sub-trait covers a distinct Docker resource domain. Mocks implement only
//! the sub-traits they exercise. The composed [`DockerClient`] is the surface that
//! action beads (B8–B13) depend on — object-safe so it can be used as
//! `&dyn DockerClient` in free-function action seams.

use std::pin::Pin;

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
use futures_util::Stream;

use anyhow::Result;

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

    /// Liveness probe. Used by [`DockerClientCache`](crate::docker_client::DockerClientCache)
    /// for explicit health checks and by B8/B9 to detect a dead tunnel.
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
