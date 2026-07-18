use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use std::path::{Component, Path};

mod command_policy;

pub(crate) use command_policy::command_filesystem_operand_indices;
pub use command_policy::{
    ALLOWED_READ_COMMANDS, EXEC_DENYLIST, validate_command, validate_command_args,
};

#[cfg(test)]
#[path = "synapse_tests.rs"]
mod tests;

#[non_exhaustive]
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum HostProtocol {
    Local,
    Ssh,
    Http,
    Https,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct HostConfig {
    pub name: String,
    pub host: String,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default = "default_protocol")]
    pub protocol: HostProtocol,
    #[serde(rename = "sshUser", default)]
    pub ssh_user: Option<String>,
    #[serde(rename = "sshKeyPath", default)]
    pub ssh_key_path: Option<String>,
    #[serde(rename = "sshPort", default)]
    pub ssh_port: Option<u16>,
    #[serde(rename = "sshConfigPath", default)]
    pub ssh_config_path: Option<String>,
    #[serde(rename = "dockerSocketPath", default)]
    pub docker_socket_path: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(rename = "composeSearchPaths", default)]
    pub compose_search_paths: Vec<String>,
    #[serde(rename = "scoutReadRoots", default)]
    pub scout_read_roots: Vec<String>,
    #[serde(rename = "execAllowlist", default)]
    pub exec_allowlist: Vec<String>,
}

impl HostConfig {
    pub fn local() -> Self {
        Self {
            name: "local".into(),
            host: "localhost".into(),
            port: None,
            protocol: HostProtocol::Local,
            ssh_user: None,
            ssh_key_path: None,
            ssh_port: None,
            ssh_config_path: None,
            docker_socket_path: Some("/var/run/docker.sock".into()),
            tags: vec!["local".into()],
            compose_search_paths: Vec::new(),
            scout_read_roots: vec!["/tmp".into()],
            exec_allowlist: Vec::new(),
        }
    }

    /// Complete identity for every transport-affecting topology field.
    /// Caches must never key only by alias: aliases can be retargeted at runtime.
    pub fn connection_key(&self) -> String {
        format!(
            "{}|{:?}|{}|{}|{}|{}|{}|{}|{}",
            self.name,
            self.protocol,
            self.host,
            self.port.map(|v| v.to_string()).unwrap_or_default(),
            self.ssh_user.as_deref().unwrap_or_default(),
            self.ssh_port.map(|v| v.to_string()).unwrap_or_default(),
            self.ssh_key_path.as_deref().unwrap_or_default(),
            self.ssh_config_path.as_deref().unwrap_or_default(),
            self.docker_socket_path.as_deref().unwrap_or_default(),
        )
    }
}

fn default_protocol() -> HostProtocol {
    HostProtocol::Local
}

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct HostsFile {
    pub hosts: Vec<HostConfig>,
}

pub fn validate_safe_path(path: &str) -> Result<()> {
    if path.is_empty() {
        bail!("path must not be empty");
    }

    // SECURITY FIX: Require absolute path (starts with /)
    if !path.starts_with('/') {
        bail!("absolute path required");
    }

    if path.split('/').any(|part| part == "..") {
        bail!("path traversal is not allowed");
    }
    if !path
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '.' | '_' | '-'))
    {
        bail!("path contains unsafe characters");
    }

    // SECURITY FIX: Reject symlinks via symlink_metadata before any read.
    // std::fs::read_to_string follows symlinks — this protects against
    // symlink-based arbitrary file reads in world-writable directories.
    match std::fs::symlink_metadata(path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() {
                bail!("symlinks not permitted");
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // Path doesn't exist yet — this is OK (e.g., during file creation).
            // The actual operation (read/write) will check existence.
        }
        Err(e) => bail!("cannot validate path: {e}"),
    }

    Ok(())
}

pub fn validate_scout_read_path(host: &HostConfig, path: &str) -> Result<()> {
    validate_safe_path(path)?;
    reject_sensitive_read_path(path)?;

    let roots = scout_allowed_read_roots(host);
    if roots.is_empty() {
        bail!("scout file reads are disabled for host {}", host.name);
    }

    if roots.iter().any(|root| path_is_under_root(path, root)) {
        return Ok(());
    }

    bail!(
        "path is outside configured scout read roots for host {}",
        host.name
    )
}

pub fn scout_allowed_read_roots(host: &HostConfig) -> Vec<String> {
    let mut roots = Vec::new();
    for root in host
        .scout_read_roots
        .iter()
        .chain(host.compose_search_paths.iter())
    {
        let root = if root == "/" {
            "/"
        } else {
            root.trim_end_matches('/')
        };
        if root.is_empty() {
            continue;
        }
        if validate_safe_path(root).is_err() {
            continue;
        }
        if !roots.iter().any(|existing| existing == root) {
            roots.push(root.to_owned());
        }
    }
    roots
}

fn reject_sensitive_read_path(path: &str) -> Result<()> {
    let sensitive = Path::new(path).components().any(|component| {
        let Component::Normal(part) = component else {
            return false;
        };
        let part = part.to_string_lossy();
        matches!(
            part.as_ref(),
            ".ssh"
                | ".env"
                | ".env.local"
                | ".env.production"
                | "authorized_keys"
                | "id_rsa"
                | "id_dsa"
                | "id_ecdsa"
                | "id_ed25519"
        ) || part.ends_with(".pem")
    });
    if sensitive {
        bail!("sensitive scout read path is not permitted");
    }
    Ok(())
}

fn path_is_under_root(path: &str, root: &str) -> bool {
    if root == "/" {
        return true;
    }

    let path_obj = Path::new(path);
    let root_obj = Path::new(root);
    if path_obj.exists()
        && root_obj.exists()
        && let (Ok(canonical_path), Ok(canonical_root)) = (
            std::fs::canonicalize(path_obj),
            std::fs::canonicalize(root_obj),
        )
    {
        return canonical_path.starts_with(canonical_root);
    }

    path == root
        || path
            .strip_prefix(root)
            .is_some_and(|rest| rest.starts_with('/'))
}
