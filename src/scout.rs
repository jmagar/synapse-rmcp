use anyhow::{bail, Result};
use serde_json::{json, Value};
use std::path::Path;
use std::process::Command;

use crate::host_config::HostRepository;
use crate::synapse::{validate_command, validate_safe_path, HostConfig, HostProtocol};

pub fn nodes(repo: &dyn HostRepository) -> Result<Value> {
    let hosts = repo.load_hosts()?;
    Ok(json!({ "hosts": hosts }))
}

pub fn resolve_host(repo: &dyn HostRepository, name: &str) -> Result<HostConfig> {
    repo.load_hosts()?
        .into_iter()
        .find(|host| host.name == name)
        .ok_or_else(|| anyhow::anyhow!("unknown host: {name}"))
}

pub fn peek(repo: &dyn HostRepository, host_name: &str, path: &str) -> Result<Value> {
    validate_safe_path(path)?;
    let host = resolve_host(repo, host_name)?;
    if host.protocol != HostProtocol::Local && host.host != "localhost" {
        bail!("scout peek remote hosts are deferred in synapse2 MVP");
    }
    let metadata = std::fs::metadata(path)?;
    if metadata.is_dir() {
        let entries = std::fs::read_dir(path)?
            .filter_map(Result::ok)
            .take(100)
            .map(|entry| entry.file_name().to_string_lossy().to_string())
            .collect::<Vec<_>>();
        return Ok(json!({
            "host": host.name,
            "path": path,
            "kind": "directory",
            "entries": entries,
        }));
    }
    let text = std::fs::read_to_string(path)?;
    Ok(json!({
        "host": host.name,
        "path": path,
        "kind": "file",
        "content": text,
    }))
}

pub fn exec(
    repo: &dyn HostRepository,
    host_name: &str,
    path: &str,
    command: &str,
) -> Result<Value> {
    validate_safe_path(path)?;
    let host = resolve_host(repo, host_name)?;
    validate_command(command, &host.exec_allowlist)?;
    if host.protocol != HostProtocol::Local && host.host != "localhost" {
        bail!("scout exec remote hosts are deferred in synapse2 MVP");
    }
    let output = Command::new(command)
        .current_dir(Path::new(path))
        .output()?;
    Ok(json!({
        "host": host.name,
        "path": path,
        "command": command,
        "exit_code": output.status.code(),
        "stdout": String::from_utf8_lossy(&output.stdout),
        "stderr": String::from_utf8_lossy(&output.stderr),
    }))
}
