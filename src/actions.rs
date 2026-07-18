use anyhow::Result;
use serde_json::{Value, json};

// ── Submodules ────────────────────────────────────────────────────────────────

mod dispatch;
mod flux;
pub mod operations;
pub mod rest;
pub(crate) mod scout;

// ── Re-exports (keep crate::actions::X resolving for all callers) ─────────────

pub use dispatch::{execute_service_action, is_confirmation_denied, is_validation_error};
pub use flux::{ComposeArgs, ContainerArgs, DockerArgs, HostArgs};
pub use operations::{
    OPERATION_SPECS, OperationSpec, OperationTool, OperationTransport, operation,
    operation_for_shape, operations_for_tool,
};
pub use scout::{
    ScoutBeamArgs, ScoutDeltaArgs, ScoutEmitArgs, ScoutEmitTarget, ScoutExecArgs, ScoutFindArgs,
    ScoutLogsArgs, ScoutPsArgs, ScoutZfsArgs,
};

// ── Validation error type ─────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("action is required")]
    MissingAction,
    #[error("`{field}` is required and must not be empty")]
    MissingField { field: String },
    #[error("`{field}` must be a string")]
    WrongType { field: String },
    #[error("action={action} is not available over REST; use MCP or action=help for documentation")]
    NotAvailableOverRest { action: String },
    #[error("unknown synapse2 action: {action}; use action=help for documentation")]
    UnknownAction { action: String },
}

// ── Scope constants & helpers ─────────────────────────────────────────────────

pub const READ_SCOPE: &str = "synapse:read";
pub const WRITE_SCOPE: &str = "synapse:write";
pub const DENY_SCOPE: &str = "synapse2:__deny__";

/// Returns true if `token_scopes` satisfy `required`.
/// Write scope satisfies read (write ⊇ read).
/// Single source of truth — called from both REST and MCP enforcement paths.
pub fn scopes_satisfy(token_scopes: &[String], required: &str) -> bool {
    token_scopes
        .iter()
        .any(|s| s == required || (required == READ_SCOPE && s == WRITE_SCOPE))
}

pub fn action_names() -> Vec<&'static str> {
    let mut names = Vec::new();
    for spec in OPERATION_SPECS {
        if !names.contains(&spec.action) {
            names.push(spec.action);
        }
    }
    names
}

pub fn is_known_action(action: &str) -> bool {
    OPERATION_SPECS.iter().any(|spec| spec.action == action)
}

pub fn rest_action_names() -> Vec<&'static str> {
    rest::action_names()
}

pub fn is_rest_action(action: &str) -> bool {
    rest::operation(action).is_some()
}

pub fn mcp_operation_names() -> Vec<&'static str> {
    OPERATION_SPECS.iter().map(|spec| spec.name).collect()
}

pub fn mcp_only_action_names() -> Vec<&'static str> {
    OPERATION_SPECS
        .iter()
        .filter(|spec| spec.transport == OperationTransport::McpOnly)
        .map(|spec| spec.name)
        .collect()
}

pub fn required_scope_for_action(action: &str) -> Option<&'static str> {
    let mut matches = OPERATION_SPECS.iter().filter(|spec| spec.action == action);
    let Some(first) = matches.next() else {
        return Some(DENY_SCOPE);
    };
    first.required_scope?;
    if first.required_scope == Some(READ_SCOPE)
        || matches.any(|spec| spec.required_scope == Some(READ_SCOPE))
    {
        Some(READ_SCOPE)
    } else {
        Some(WRITE_SCOPE)
    }
}

/// Derive scope from the parsed action, including flux subactions.
///
/// Top-level flux actions mix read-only and mutating subactions behind one
/// MCP/CLI action name, so mounted auth must authorize the parsed shape rather
/// than the raw `action` string.
pub fn required_scope_for_parsed_action(action: &SynapseAction) -> Option<&'static str> {
    let (tool, subaction) = match action {
        SynapseAction::FluxDocker(args) => (OperationTool::Flux, Some(args.subaction.as_str())),
        SynapseAction::FluxContainer(args) => (OperationTool::Flux, Some(args.subaction.as_str())),
        SynapseAction::FluxHost(args) => (OperationTool::Flux, Some(args.subaction.as_str())),
        SynapseAction::FluxCompose(args) => (OperationTool::Flux, Some(args.subaction.as_str())),
        SynapseAction::FluxHelp { .. } => (OperationTool::Flux, None),
        SynapseAction::ScoutHelp { .. } => (OperationTool::Scout, None),
        SynapseAction::ScoutZfs(args) => (OperationTool::Scout, Some(args.subaction.as_str())),
        SynapseAction::ScoutLogs(args) => (OperationTool::Scout, Some(args.subaction.as_str())),
        _ => (OperationTool::Scout, None),
    };
    operation_for_shape(tool, action.name(), subaction)
        .map(|spec| spec.required_scope)
        .unwrap_or(Some(DENY_SCOPE))
}

// ── SynapseAction enum ────────────────────────────────────────────────────────

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SynapseAction {
    /// B16: topic-aware help for the flux tool.
    /// `topic=None` → index; `topic=Some(t)` → per-subaction docs.
    FluxHelp {
        topic: Option<String>,
        format: Option<String>,
    },
    FluxDocker(Box<DockerArgs>),
    FluxContainer(Box<ContainerArgs>),
    FluxHost(Box<HostArgs>),
    FluxCompose(Box<ComposeArgs>),
    /// B16: topic-aware help for the scout tool.
    ScoutHelp {
        topic: Option<String>,
        format: Option<String>,
    },
    ScoutNodes,
    ScoutPeek {
        host: String,
        path: String,
        tree: bool,
        depth: u8,
    },
    ScoutFind(Box<ScoutFindArgs>),
    ScoutPs(Box<ScoutPsArgs>),
    ScoutDf {
        host: String,
        path: Option<String>,
    },
    ScoutDelta(Box<ScoutDeltaArgs>),
    ScoutExec(Box<ScoutExecArgs>),
    ScoutEmit(Box<ScoutEmitArgs>),
    ScoutBeam(Box<ScoutBeamArgs>),
    /// B15: ZFS subactions (pools/datasets/snapshots).
    ScoutZfs(Box<ScoutZfsArgs>),
    /// B15: Log subactions (syslog/journal/dmesg/auth).
    ScoutLogs(Box<ScoutLogsArgs>),
}

impl SynapseAction {
    pub fn name(&self) -> &'static str {
        match self {
            Self::FluxHelp { .. } | Self::ScoutHelp { .. } => "help",
            Self::FluxDocker(_) => "docker",
            Self::FluxContainer(_) => "container",
            Self::FluxHost(_) => "host",
            Self::FluxCompose(_) => "compose",
            Self::ScoutNodes => "nodes",
            Self::ScoutPeek { .. } => "peek",
            Self::ScoutFind(_) => "find",
            Self::ScoutPs(_) => "ps",
            Self::ScoutDf { .. } => "df",
            Self::ScoutDelta(_) => "delta",
            Self::ScoutExec(_) => "exec",
            Self::ScoutEmit(_) => "emit",
            Self::ScoutBeam(_) => "beam",
            Self::ScoutZfs(_) => "zfs",
            Self::ScoutLogs(_) => "logs",
        }
    }
}

// ── REST help ─────────────────────────────────────────────────────────────────

pub fn rest_help() -> Value {
    json!({
        "actions": rest::action_names(),
        "mcp_only_actions": mcp_only_action_names(),
        "usage": "Use MCP tools `flux` and `scout`, or CLI commands `synapse flux ...` and `synapse scout ...`.",
        "examples": {
            "flux":  {"action": "docker", "subaction": "info"},
            "scout": {"action": "nodes"},
        }
    })
}

// ── Shared param helpers (used by flux.rs and scout.rs via super::) ───────────

pub(crate) fn required_string_param(params: &Value, name: &str) -> Result<String> {
    optional_string_param(params, name)?
        .filter(|value| !value.is_empty())
        .ok_or_else(|| ValidationError::MissingField { field: name.into() }.into())
}

pub(crate) fn optional_string_param(params: &Value, name: &str) -> Result<Option<String>> {
    match params.get(name) {
        None => Ok(None),
        Some(value) => value
            .as_str()
            .map(|s| Some(s.to_owned()))
            .ok_or_else(|| ValidationError::WrongType { field: name.into() }.into()),
    }
}

/// Require a non-empty optional string field, returning a `MissingField`
/// validation error when absent or empty.
pub(crate) fn require_field<'a>(value: &'a Option<String>, name: &str) -> Result<&'a str> {
    value
        .as_deref()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| ValidationError::MissingField { field: name.into() }.into())
}

/// Require a `container_id` for single-container subactions.
pub(crate) fn require_container_id(container_id: &Option<String>) -> Result<&str> {
    container_id
        .as_deref()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            ValidationError::MissingField {
                field: "container_id".into(),
            }
            .into()
        })
}

pub(crate) fn optional_bool_param(params: &Value, name: &str) -> Result<Option<bool>> {
    match params.get(name) {
        None => Ok(None),
        Some(value) => value
            .as_bool()
            .map(Some)
            .ok_or_else(|| ValidationError::WrongType { field: name.into() }.into()),
    }
}

pub(crate) fn optional_u32_param(params: &Value, name: &str) -> Result<Option<u32>> {
    match params.get(name) {
        None => Ok(None),
        Some(value) => value
            .as_u64()
            .and_then(|v| u32::try_from(v).ok())
            .map(Some)
            .ok_or_else(|| ValidationError::WrongType { field: name.into() }.into()),
    }
}

pub(crate) fn optional_u64_param(params: &Value, name: &str) -> Result<Option<u64>> {
    match params.get(name) {
        None => Ok(None),
        Some(value) => value
            .as_u64()
            .map(Some)
            .ok_or_else(|| ValidationError::WrongType { field: name.into() }.into()),
    }
}

/// Extract an optional array of strings from `params[name]`.
/// Returns an empty `Vec` when the key is absent; errors on type mismatch.
pub(crate) fn optional_string_array_param(params: &Value, name: &str) -> Result<Vec<String>> {
    match params.get(name) {
        None => Ok(Vec::new()),
        Some(Value::Array(arr)) => arr
            .iter()
            .map(|v| {
                v.as_str().map(|s| s.to_owned()).ok_or_else(|| {
                    ValidationError::WrongType {
                        field: format!("{name}[]"),
                    }
                    .into()
                })
            })
            .collect(),
        Some(_) => Err(ValidationError::WrongType { field: name.into() }.into()),
    }
}

#[cfg(test)]
#[path = "actions_tests.rs"]
mod tests;
