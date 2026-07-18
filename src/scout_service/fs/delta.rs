use std::io::Read;

use anyhow::{Result, bail};
use serde_json::{Value, json};

use super::REMOTE_READ_SCRIPT;
use crate::flux_service::host::is_local_host;
use crate::ssh::SshExecutor;
use crate::synapse::{HostConfig, validate_scout_read_path};

/// Maximum inline content size for delta content mode.
pub const DELTA_MAX_CONTENT_BYTES: usize = 1024 * 1024;

/// Compare a remote file against either another remote file or inline content.
///
/// `source` — `{host, path}` of the file to read.
/// `target` — optional `{host, path}` to diff against.
/// `content` — optional inline string (capped at 1 MB).
///
/// Exactly one of `target` or `content` must be supplied.
pub async fn delta(
    source_host: &HostConfig,
    executor: &dyn SshExecutor,
    source_path: &str,
    target_host: Option<&HostConfig>,
    target_path: Option<&str>,
    content: Option<&str>,
) -> Result<Value> {
    validate_scout_read_path(source_host, source_path)?;

    // VALIDATION FIRST — content size checked before any IO.
    if let Some(inline) = content
        && inline.len() > DELTA_MAX_CONTENT_BYTES
    {
        bail!("delta content exceeds 1 MB limit");
    }

    match (target_host, target_path, content) {
        (Some(th), Some(tp), None) => {
            validate_scout_read_path(th, tp)?;
            let source_content = read_remote_file(source_host, executor, source_path).await?;
            let source_label = format!("{}:{}", source_host.name, source_path);
            let target_content = read_remote_file(th, executor, tp).await?;
            let target_label = format!("{}:{}", th.name, tp);
            let diff = bounded_diff(
                source_content,
                target_content,
                source_label.clone(),
                target_label.clone(),
            )
            .await?;
            Ok(json!({
                "identical": diff.is_empty(),
                "source": source_label,
                "target": target_label,
                "diff": diff,
            }))
        }
        (None, None, Some(inline)) => {
            let source_content = read_remote_file(source_host, executor, source_path).await?;
            let source_label = format!("{}:{}", source_host.name, source_path);
            let diff = bounded_diff(
                source_content,
                inline.to_owned(),
                source_label.clone(),
                "inline".into(),
            )
            .await?;
            Ok(json!({
                "identical": diff.is_empty(),
                "source": source_label,
                "target": "inline",
                "diff": diff,
            }))
        }
        _ => bail!("delta requires exactly one of: target or content"),
    }
}

/// Read through a descriptor walk that rejects symlinks at every path segment.
async fn read_remote_file(
    host: &HostConfig,
    executor: &dyn SshExecutor,
    path: &str,
) -> Result<String> {
    if is_local_host(host) {
        validate_scout_read_path(host, path)?;
        let host = host.clone();
        let path = path.to_owned();
        tokio::task::spawn_blocking(move || {
            read_local_bounded(&host, &path, DELTA_MAX_CONTENT_BYTES)
        })
        .await?
    } else {
        let (root, relative) = crate::secure_path::root_and_relative(host, path)?;
        let count = DELTA_MAX_CONTENT_BYTES.to_string();
        let out = executor
            .exec(
                host,
                "python3",
                &["-c", REMOTE_READ_SCRIPT, "read", &root, &relative, &count],
            )
            .await?;
        if out.exit_code != Some(0) && !out.stderr.is_empty() {
            bail!("read {path}: {}", out.stderr.trim());
        }
        Ok(out.stdout)
    }
}

fn read_local_bounded(host: &HostConfig, path: &str, cap: usize) -> Result<String> {
    let bound = crate::secure_path::bind_read_path(host, path)?;
    let metadata = bound.file().metadata()?;
    if metadata.len() > cap as u64 {
        bail!("delta file exceeds 1 MB limit: {path}");
    }
    let mut reader = bound.into_file().take((cap + 1) as u64);
    let mut content = String::new();
    reader.read_to_string(&mut content)?;
    if content.len() > cap {
        bail!("delta file exceeds 1 MB limit: {path}");
    }
    Ok(content)
}

async fn bounded_diff(a: String, b: String, label_a: String, label_b: String) -> Result<String> {
    tokio::task::spawn_blocking(move || compute_diff(&a, &b, &label_a, &label_b))
        .await
        .map_err(Into::into)
}
/// Compute a unified diff between `a` and `b`, labelled by `label_a`/`label_b`.
///
/// Pure function — no IO. Returns empty string when files are identical.
pub fn compute_diff(a: &str, b: &str, label_a: &str, label_b: &str) -> String {
    if a == b {
        return String::new();
    }

    // Line-by-line diff (simple unified format without the patch header offsets).
    let a_lines: Vec<&str> = a.lines().collect();
    let b_lines: Vec<&str> = b.lines().collect();
    let a_set: std::collections::HashSet<&str> = a_lines.iter().copied().collect();
    let b_set: std::collections::HashSet<&str> = b_lines.iter().copied().collect();

    let mut result = format!("--- {label_a}\n+++ {label_b}\n");
    let mut remaining =
        crate::runtime_budget::SERVICE_TEXT_FIELD_BYTE_CAP.saturating_sub(result.len());

    // Naive diff: mark lines removed from a, added in b.
    // For parity we just produce a simple two-column representation.
    // A full Myers diff is out of scope; the format matches synapse-mcp's
    // "Files differ" indicator at the service layer.
    for line in &a_lines {
        if !b_set.contains(line) {
            let line = format!("- {line}\n");
            if line.len() > remaining {
                break;
            }
            result.push_str(&line);
            remaining -= line.len();
        }
    }
    for line in &b_lines {
        if !a_set.contains(line) {
            let line = format!("+ {line}\n");
            if line.len() > remaining {
                break;
            }
            result.push_str(&line);
            remaining -= line.len();
        }
    }

    result
}
