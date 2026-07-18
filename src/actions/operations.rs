//! Canonical operation-level metadata shared by MCP, REST, schemas, and docs.

use super::{READ_SCOPE, WRITE_SCOPE};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationTool {
    Flux,
    Scout,
    Both,
}

impl OperationTool {
    pub fn supports(self, tool: OperationTool) -> bool {
        self == OperationTool::Both || self == tool
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationTransport {
    Rest,
    McpOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OperationSpec {
    pub name: &'static str,
    pub tool: OperationTool,
    pub action: &'static str,
    pub subaction: Option<&'static str>,
    pub required_scope: Option<&'static str>,
    pub destructive: bool,
    pub transport: OperationTransport,
    pub required_params: &'static [&'static str],
    /// Alternative complete parameter groups, used by operations such as delta.
    pub required_any: &'static [&'static [&'static str]],
}

macro_rules! operation {
    ($name:literal, $tool:ident, $action:literal, $subaction:expr, $scope:expr, $destructive:literal, $transport:ident, [$($required:literal),* $(,)?]) => {
        OperationSpec {
            name: $name,
            tool: OperationTool::$tool,
            action: $action,
            subaction: $subaction,
            required_scope: $scope,
            destructive: $destructive,
            transport: OperationTransport::$transport,
            required_params: &[$($required),*],
            required_any: &[],
        }
    };
    ($name:literal, $tool:ident, $action:literal, $subaction:expr, $scope:expr, $destructive:literal, $transport:ident, [$($required:literal),* $(,)?], any [$([$($alternative:literal),+ $(,)?]),+ $(,)?]) => {
        OperationSpec {
            name: $name,
            tool: OperationTool::$tool,
            action: $action,
            subaction: $subaction,
            required_scope: $scope,
            destructive: $destructive,
            transport: OperationTransport::$transport,
            required_params: &[$($required),*],
            required_any: &[$(&[$($alternative),+]),+],
        }
    };
}

#[rustfmt::skip]
pub const OPERATION_SPECS: &[OperationSpec] = &[
    operation!("help", Both, "help", None, None, false, Rest, []),
    operation!("flux.docker.info", Flux, "docker", Some("info"), Some(READ_SCOPE), false, Rest, []),
    operation!("flux.docker.df", Flux, "docker", Some("df"), Some(READ_SCOPE), false, Rest, []),
    operation!("flux.docker.images", Flux, "docker", Some("images"), Some(READ_SCOPE), false, Rest, []),
    operation!("flux.docker.networks", Flux, "docker", Some("networks"), Some(READ_SCOPE), false, Rest, []),
    operation!("flux.docker.volumes", Flux, "docker", Some("volumes"), Some(READ_SCOPE), false, Rest, []),
    operation!("flux.docker.pull", Flux, "docker", Some("pull"), Some(WRITE_SCOPE), false, Rest, ["host", "image"]),
    operation!("flux.docker.build", Flux, "docker", Some("build"), Some(WRITE_SCOPE), true, Rest, ["host", "context", "tag"]),
    operation!("flux.docker.rmi", Flux, "docker", Some("rmi"), Some(WRITE_SCOPE), true, Rest, ["host", "image", "force"]),
    operation!("flux.docker.prune", Flux, "docker", Some("prune"), Some(WRITE_SCOPE), true, Rest, ["host", "prune_target", "force"]),
    operation!("flux.container.list", Flux, "container", Some("list"), Some(READ_SCOPE), false, Rest, []),
    operation!("flux.container.inspect", Flux, "container", Some("inspect"), Some(READ_SCOPE), false, McpOnly, ["container_id"]),
    operation!("flux.container.logs", Flux, "container", Some("logs"), Some(READ_SCOPE), false, McpOnly, ["container_id"]),
    operation!("flux.container.stats", Flux, "container", Some("stats"), Some(READ_SCOPE), false, McpOnly, []),
    operation!("flux.container.top", Flux, "container", Some("top"), Some(READ_SCOPE), false, McpOnly, ["container_id"]),
    operation!("flux.container.search", Flux, "container", Some("search"), Some(READ_SCOPE), false, McpOnly, ["query"]),
    operation!("flux.container.start", Flux, "container", Some("start"), Some(WRITE_SCOPE), false, McpOnly, ["host", "container_id"]),
    operation!("flux.container.stop", Flux, "container", Some("stop"), Some(WRITE_SCOPE), true, McpOnly, ["host", "container_id"]),
    operation!("flux.container.restart", Flux, "container", Some("restart"), Some(WRITE_SCOPE), false, McpOnly, ["host", "container_id"]),
    operation!("flux.container.pause", Flux, "container", Some("pause"), Some(WRITE_SCOPE), false, McpOnly, ["host", "container_id"]),
    operation!("flux.container.resume", Flux, "container", Some("resume"), Some(WRITE_SCOPE), false, McpOnly, ["host", "container_id"]),
    operation!("flux.container.pull", Flux, "container", Some("pull"), Some(WRITE_SCOPE), false, McpOnly, ["host", "container_id"]),
    operation!("flux.container.recreate", Flux, "container", Some("recreate"), Some(WRITE_SCOPE), true, McpOnly, ["host", "container_id"]),
    operation!("flux.container.exec", Flux, "container", Some("exec"), Some(WRITE_SCOPE), true, McpOnly, ["host", "container_id", "command"]),
    operation!("flux.host.status", Flux, "host", Some("status"), Some(READ_SCOPE), false, McpOnly, []),
    operation!("flux.host.info", Flux, "host", Some("info"), Some(READ_SCOPE), false, McpOnly, []),
    operation!("flux.host.uptime", Flux, "host", Some("uptime"), Some(READ_SCOPE), false, McpOnly, []),
    operation!("flux.host.resources", Flux, "host", Some("resources"), Some(READ_SCOPE), false, McpOnly, []),
    operation!("flux.host.services", Flux, "host", Some("services"), Some(READ_SCOPE), false, McpOnly, ["host"]),
    operation!("flux.host.network", Flux, "host", Some("network"), Some(READ_SCOPE), false, McpOnly, []),
    operation!("flux.host.mounts", Flux, "host", Some("mounts"), Some(READ_SCOPE), false, McpOnly, ["host"]),
    operation!("flux.host.ports", Flux, "host", Some("ports"), Some(READ_SCOPE), false, McpOnly, ["host"]),
    operation!("flux.host.doctor", Flux, "host", Some("doctor"), Some(READ_SCOPE), false, McpOnly, ["host"]),
    operation!("flux.compose.list", Flux, "compose", Some("list"), Some(READ_SCOPE), false, McpOnly, ["host"]),
    operation!("flux.compose.status", Flux, "compose", Some("status"), Some(READ_SCOPE), false, McpOnly, ["host", "project"]),
    operation!("flux.compose.up", Flux, "compose", Some("up"), Some(WRITE_SCOPE), false, McpOnly, ["host", "project"]),
    operation!("flux.compose.down", Flux, "compose", Some("down"), Some(WRITE_SCOPE), true, McpOnly, ["host", "project"]),
    operation!("flux.compose.restart", Flux, "compose", Some("restart"), Some(WRITE_SCOPE), true, McpOnly, ["host", "project"]),
    operation!("flux.compose.recreate", Flux, "compose", Some("recreate"), Some(WRITE_SCOPE), true, McpOnly, ["host", "project"]),
    operation!("flux.compose.logs", Flux, "compose", Some("logs"), Some(READ_SCOPE), false, McpOnly, ["host", "project"]),
    operation!("flux.compose.build", Flux, "compose", Some("build"), Some(WRITE_SCOPE), false, McpOnly, ["host", "project"]),
    operation!("flux.compose.pull", Flux, "compose", Some("pull"), Some(WRITE_SCOPE), false, McpOnly, ["host", "project"]),
    operation!("flux.compose.refresh", Flux, "compose", Some("refresh"), Some(READ_SCOPE), false, McpOnly, ["host"]),
    operation!("scout.nodes", Scout, "nodes", None, Some(READ_SCOPE), false, Rest, []),
    operation!("scout.peek", Scout, "peek", None, Some(READ_SCOPE), false, Rest, ["host", "path"]),
    operation!("scout.find", Scout, "find", None, Some(READ_SCOPE), false, McpOnly, ["host", "path", "pattern"]),
    operation!("scout.ps", Scout, "ps", None, Some(READ_SCOPE), false, McpOnly, ["host"]),
    operation!("scout.df", Scout, "df", None, Some(READ_SCOPE), false, McpOnly, ["host"]),
    operation!("scout.delta", Scout, "delta", None, Some(READ_SCOPE), false, McpOnly, ["source_host", "source_path"], any [["content"], ["target_host", "target_path"]]),
    operation!("scout.exec", Scout, "exec", None, Some(WRITE_SCOPE), true, Rest, ["host", "command"]),
    operation!("scout.emit", Scout, "emit", None, Some(WRITE_SCOPE), true, McpOnly, ["targets", "command"]),
    operation!("scout.beam", Scout, "beam", None, Some(WRITE_SCOPE), true, McpOnly, ["source_host", "source_path", "dest_host", "dest_path"]),
    operation!("scout.zfs.pools", Scout, "zfs", Some("pools"), Some(READ_SCOPE), false, McpOnly, ["host"]),
    operation!("scout.zfs.datasets", Scout, "zfs", Some("datasets"), Some(READ_SCOPE), false, McpOnly, ["host"]),
    operation!("scout.zfs.snapshots", Scout, "zfs", Some("snapshots"), Some(READ_SCOPE), false, McpOnly, ["host"]),
    operation!("scout.logs.syslog", Scout, "logs", Some("syslog"), Some(READ_SCOPE), false, McpOnly, ["host"]),
    operation!("scout.logs.journal", Scout, "logs", Some("journal"), Some(READ_SCOPE), false, McpOnly, ["host"]),
    operation!("scout.logs.dmesg", Scout, "logs", Some("dmesg"), Some(READ_SCOPE), false, McpOnly, ["host"]),
    operation!("scout.logs.auth", Scout, "logs", Some("auth"), Some(READ_SCOPE), false, McpOnly, ["host"]),
];

pub fn operation(name: &str) -> Option<&'static OperationSpec> {
    OPERATION_SPECS.iter().find(|spec| spec.name == name)
}

pub fn operation_for_shape(
    tool: OperationTool,
    action: &str,
    subaction: Option<&str>,
) -> Option<&'static OperationSpec> {
    OPERATION_SPECS.iter().find(|spec| {
        spec.tool.supports(tool) && spec.action == action && spec.subaction == subaction
    })
}

pub fn operations_for_tool(tool: OperationTool) -> impl Iterator<Item = &'static OperationSpec> {
    OPERATION_SPECS
        .iter()
        .filter(move |spec| spec.tool.supports(tool))
}
