//! Flux-domain arg structs, `from_flux_args`, and dispatch helpers.
//!
//! All items here are re-exported from the parent [`crate::actions`] module so
//! call sites need no changes.

use anyhow::Result;
use serde_json::Value;

use super::{
    ValidationError, optional_bool_param, optional_string_array_param, optional_string_param,
    optional_u32_param, optional_u64_param, required_string_param,
};

mod compose;
mod container;
mod docker;
mod host;

pub(super) use compose::dispatch_flux_compose;
pub(super) use container::dispatch_flux_container;
pub(super) use docker::dispatch_flux_docker;
pub(super) use host::dispatch_flux_host;

// ── Arg structs ───────────────────────────────────────────────────────────────

/// Parsed parameters for `flux container` subactions.
///
/// Boxed inside [`super::SynapseAction::FluxContainer`] (and mirrored by the
/// CLI `Command`) so the enum stays small — every read-only container
/// subaction's params live here. Extraction stays in the shim; logic lives in
/// `FluxService`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ContainerArgs {
    pub response_format: Option<String>,
    pub subaction: String,
    pub container_id: Option<String>,
    pub host: Option<String>,
    pub lines: Option<u32>,
    // list filters
    pub state: Option<String>,
    pub name_filter: Option<String>,
    pub image_filter: Option<String>,
    pub label_filter: Option<String>,
    // logs params
    pub since: Option<String>,
    pub until: Option<String>,
    pub grep: Option<String>,
    pub stream: Option<String>,
    // inspect param
    pub summary: Option<bool>,
    // search param
    pub query: Option<String>,
    // B9: lifecycle params
    /// exec: command as argv (index 0 = binary, no shell). Required for exec.
    /// Empty when not provided.
    pub command: Vec<String>,
    /// exec: optional user to run as.
    pub exec_user: Option<String>,
    /// exec: optional working directory inside container.
    pub exec_workdir: Option<String>,
    /// exec: timeout in ms, clamped [1000, 300000], default 30000.
    pub exec_timeout_ms: Option<u64>,
    /// recreate: whether to pull the image before recreating (default true).
    pub pull: Option<bool>,
}

/// Parsed parameters for `flux docker` subactions.
///
/// Boxed inside [`super::SynapseAction::FluxDocker`] (and mirrored by the
/// CLI) so the enum stays small. Extraction stays in the shim; all logic
/// (validation, fanout, gating) lives in `FluxService` / the `docker`
/// submodule.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DockerArgs {
    pub response_format: Option<String>,
    pub subaction: String,
    pub host: Option<String>,
    // images
    pub dangling_only: Option<bool>,
    // pull / rmi / build
    pub image: Option<String>,
    pub force: Option<bool>,
    pub context: Option<String>,
    pub tag: Option<String>,
    pub dockerfile: Option<String>,
    pub no_cache: Option<bool>,
    // prune
    pub prune_target: Option<String>,
}

/// Parsed parameters for `flux host` subactions (B11).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HostArgs {
    pub response_format: Option<String>,
    pub subaction: String,
    /// Target host name (None = fan out to all hosts).
    pub host: Option<String>,
    // services params
    pub state: Option<String>,
    pub service: Option<String>,
    // ports params
    pub protocol: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    // doctor params
    pub checks: Option<String>, // comma-separated check names
}

/// Parsed parameters for `flux compose` subactions (B13).
///
/// Boxed inside [`super::SynapseAction::FluxCompose`] so the enum stays
/// small. Extraction lives in the shim; all logic lives in `FluxService`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ComposeArgs {
    pub response_format: Option<String>,
    /// Subaction: list|status|up|down|restart|recreate|logs|build|pull|refresh.
    pub subaction: String,
    /// Target host name. Required for all subactions except `list` (where it
    /// is also required — compose ops are always single-host).
    pub host: Option<String>,
    /// Compose project name. Required for all subactions except
    /// `list`/`refresh`.
    pub project: Option<String>,
    // down params
    pub remove_volumes: Option<bool>,
    pub force: Option<bool>,
    // logs params
    pub lines: Option<u32>,
    pub since: Option<String>,
    /// Single service filter for `logs`/`status`.
    pub service: Option<String>,
    // build/pull: same `service` field above
}

// ── from_flux_args ─────────────────────────────────────────────────────────

impl super::SynapseAction {
    pub fn from_flux_args(args: &Value) -> Result<Self> {
        let action = args
            .get("action")
            .and_then(Value::as_str)
            .ok_or(ValidationError::MissingAction)?;
        match action {
            "help" => Ok(Self::FluxHelp {
                topic: optional_string_param(args, "topic")?,
                format: optional_string_param(args, "format")?,
            }),
            "docker" => Ok(Self::FluxDocker(Box::new(DockerArgs {
                response_format: optional_string_param(args, "response_format")?,
                subaction: required_string_param(args, "subaction")?,
                host: optional_string_param(args, "host")?,
                dangling_only: optional_bool_param(args, "dangling_only")?,
                image: optional_string_param(args, "image")?,
                force: optional_bool_param(args, "force")?,
                context: optional_string_param(args, "context")?,
                tag: optional_string_param(args, "tag")?,
                dockerfile: optional_string_param(args, "dockerfile")?,
                no_cache: optional_bool_param(args, "no_cache")?,
                prune_target: optional_string_param(args, "prune_target")?,
            }))),
            "container" => {
                // Validate `response_format` at the shim per B4 contract (no-op
                // on output shape today; full rendering wiring is a separate
                // codebase-wide concern). Invalid value → hard error.
                if let Some(rf) = optional_string_param(args, "response_format")? {
                    crate::formatters::ResponseFormat::parse(Some(&rf))
                        .map_err(|e| anyhow::anyhow!(e))?;
                }
                Ok(Self::FluxContainer(Box::new(ContainerArgs {
                    response_format: optional_string_param(args, "response_format")?,
                    subaction: required_string_param(args, "subaction")?,
                    container_id: optional_string_param(args, "container_id")?,
                    host: optional_string_param(args, "host")?,
                    lines: optional_u32_param(args, "lines")?,
                    state: optional_string_param(args, "state")?,
                    name_filter: optional_string_param(args, "name_filter")?,
                    image_filter: optional_string_param(args, "image_filter")?,
                    label_filter: optional_string_param(args, "label_filter")?,
                    since: optional_string_param(args, "since")?,
                    until: optional_string_param(args, "until")?,
                    grep: optional_string_param(args, "grep")?,
                    stream: optional_string_param(args, "stream")?,
                    summary: optional_bool_param(args, "summary")?,
                    query: optional_string_param(args, "query")?,
                    // B9 lifecycle params
                    command: optional_string_array_param(args, "command")?,
                    exec_user: optional_string_param(args, "exec_user")?,
                    exec_workdir: optional_string_param(args, "exec_workdir")?,
                    exec_timeout_ms: optional_u64_param(args, "exec_timeout_ms")?,
                    pull: optional_bool_param(args, "pull")?,
                })))
            }
            "host" => Ok(Self::FluxHost(Box::new(HostArgs {
                response_format: optional_string_param(args, "response_format")?,
                subaction: required_string_param(args, "subaction")?,
                host: optional_string_param(args, "host")?,
                state: optional_string_param(args, "state")?,
                service: optional_string_param(args, "service")?,
                protocol: optional_string_param(args, "protocol")?,
                limit: optional_u32_param(args, "limit")?,
                offset: optional_u32_param(args, "offset")?,
                checks: optional_string_param(args, "checks")?,
            }))),
            "compose" => Ok(Self::FluxCompose(Box::new(ComposeArgs {
                response_format: optional_string_param(args, "response_format")?,
                subaction: required_string_param(args, "subaction")?,
                host: optional_string_param(args, "host")?,
                project: optional_string_param(args, "project")?,
                remove_volumes: optional_bool_param(args, "remove_volumes")?,
                force: optional_bool_param(args, "force")?,
                lines: optional_u32_param(args, "lines")?,
                since: optional_string_param(args, "since")?,
                service: optional_string_param(args, "service")?,
            }))),
            other => Err(ValidationError::UnknownAction {
                action: other.to_owned(),
            }
            .into()),
        }
    }
}
