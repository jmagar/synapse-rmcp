use std::fs::File;
use std::io::Read;

use anyhow::{Result, bail};
use serde_json::{Value, json};

use super::{
    PEEK_MAX_CONTENT_BYTES, REMOTE_READ_SCRIPT, REMOTE_WALK_SCRIPT, WALK_MAX_RESULTS,
    WALK_MAX_VISITED, bounded_local_walk,
};
use crate::flux_service::host::{HostExec, RemoteExec, is_local_host};
use crate::ssh::SshExecutor;
use crate::synapse::{HostConfig, validate_scout_read_path};

/// Peek at a path on `host`: returns directory listing or file content.
///
/// Parameters:
/// - `path` — absolute path (validated by `validate_safe_path`)
/// - `tree` — if true, emit a depth-limited directory tree
/// - `depth` — tree depth 1–10 (default 3)
pub async fn peek(
    host: &HostConfig,
    executor: &dyn SshExecutor,
    path: &str,
    tree: bool,
    depth: u8,
) -> Result<Value> {
    validate_scout_read_path(host, path)?;

    let depth = depth.clamp(1, 10);

    if tree {
        return peek_tree(host, executor, path, depth).await;
    }

    if is_local_host(host) {
        let host = host.clone();
        let path = path.to_owned();
        tokio::task::spawn_blocking(move || peek_local(&host, &path)).await?
    } else {
        peek_remote(host, executor, path).await
    }
}

fn peek_local(host: &HostConfig, path: &str) -> Result<Value> {
    let bound = crate::secure_path::bind_read_path(host, path)?;
    let meta = bound.file().metadata()?;
    if meta.is_dir() {
        let entries: Vec<String> = std::fs::read_dir(bound.proc_path())?
            .filter_map(Result::ok)
            .take(200)
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        Ok(json!({ "host": host.name, "path": path, "kind": "directory", "entries": entries }))
    } else {
        let (content, truncated) = read_local_preview(bound.into_file(), PEEK_MAX_CONTENT_BYTES)?;
        Ok(json!({
            "host": host.name,
            "path": path,
            "kind": "file",
            "content": content,
            "truncated": truncated,
            "size_bytes": meta.len(),
            "max_content_bytes": PEEK_MAX_CONTENT_BYTES,
        }))
    }
}

async fn peek_remote(host: &HostConfig, executor: &dyn SshExecutor, path: &str) -> Result<Value> {
    let (root, relative) = crate::secure_path::root_and_relative(host, path)?;
    let cap = PEEK_MAX_CONTENT_BYTES.to_string();
    let out = executor
        .exec(
            host,
            "python3",
            &["-c", REMOTE_READ_SCRIPT, "peek", &root, &relative, &cap],
        )
        .await?;
    if out.exit_code != Some(0) {
        bail!("peek: {}", out.stderr.trim());
    }
    let payload: Value = serde_json::from_str(out.stdout.trim())?;
    Ok(json!({
        "host": host.name, "path": path,
        "kind": payload["kind"], "entries": payload.get("entries"),
        "content": payload.get("content"), "truncated": payload.get("truncated"),
        "size_bytes": payload.get("size"), "max_content_bytes": PEEK_MAX_CONTENT_BYTES,
    }))
}

fn read_local_preview(file: File, max_bytes: usize) -> Result<(String, bool)> {
    let mut reader = file.take((max_bytes + 1) as u64);
    let mut content = String::new();
    reader.read_to_string(&mut content)?;
    Ok(truncate_preview(content, max_bytes))
}

fn truncate_preview(mut content: String, max_bytes: usize) -> (String, bool) {
    if content.len() <= max_bytes {
        return (content, false);
    }
    let mut boundary = max_bytes;
    while !content.is_char_boundary(boundary) {
        boundary -= 1;
    }
    content.truncate(boundary);
    (content, true)
}

async fn peek_tree(
    host: &HostConfig,
    executor: &dyn SshExecutor,
    path: &str,
    depth: u8,
) -> Result<Value> {
    let depth_str = depth.to_string();
    if is_local_host(host) {
        let host_name = host.name.clone();
        let host = host.clone();
        let root = path.to_owned();
        let walk =
            tokio::task::spawn_blocking(move || bounded_local_walk(&host, &root, depth)).await??;
        Ok(json!({
            "host": host_name,
            "path": path,
            "depth": depth,
            "tree": walk.items.join("\n"),
            "truncated": walk.truncated,
        }))
    } else {
        let (root, relative) = crate::secure_path::root_and_relative(host, path)?;
        let remote = RemoteExec { executor, host };
        let result_limit = WALK_MAX_RESULTS.to_string();
        let visit_limit = WALK_MAX_VISITED.to_string();
        let out = remote
            .run(
                "python3",
                &[
                    "-c",
                    REMOTE_WALK_SCRIPT,
                    "tree",
                    &root,
                    &relative,
                    path,
                    "*",
                    &depth_str,
                    &result_limit,
                    &visit_limit,
                ],
            )
            .await?;
        Ok(json!({ "host": host.name, "path": path, "depth": depth, "tree": out.stdout }))
    }
}
