//! Scout filesystem operations: bounded `peek`, `find`, and `delta` reads.

use anyhow::{Result, bail};
use serde_json::{Value, json};

#[cfg(test)]
#[path = "fs_tests.rs"]
mod tests;

use crate::flux_service::host::{HostExec, RemoteExec, is_local_host};
use crate::ssh::SshExecutor;
use crate::synapse::{HostConfig, validate_scout_read_path};

/// Maximum bytes read from a file for `peek`.
///
/// `peek` is a preview action, so this is an IO cap, not only a response cap.
/// It leaves room below the global 40 KB MCP response safety net for JSON and
/// markdown framing.
pub const PEEK_MAX_CONTENT_BYTES: usize = 32 * 1024;

mod delta;
mod peek;

pub use delta::{DELTA_MAX_CONTENT_BYTES, compute_diff, delta};
pub use peek::peek;

/// Hard result and traversal-work ceilings for filesystem walks.
pub(super) const WALK_MAX_RESULTS: usize = 500;
pub(super) const WALK_MAX_VISITED: usize = 10_000;

/// Fixed remote walker. User values are separate argv entries and never become
/// source code or shell text. The process exits as soon as `limit` is reached.
pub(super) const REMOTE_WALK_SCRIPT: &str = r#"import fnmatch, os, stat, sys
mode, root, rel, display, pattern = sys.argv[1:6]
max_depth, limit, visit_limit = map(int, sys.argv[6:9])
visited = [0]
emitted = [0]
def open_beneath(root, rel):
    fd = os.open('/', os.O_RDONLY | os.O_DIRECTORY)
    for part in [p for p in root.split('/') if p] + [p for p in rel.split('/') if p]:
        nxt = os.open(part, os.O_RDONLY | os.O_NOFOLLOW, dir_fd=fd)
        os.close(fd); fd = nxt
    return fd
def walk(fd, shown, depth):
    if visited[0] >= visit_limit or emitted[0] >= limit:
        return
    visited[0] += 1
    kind = os.fstat(fd).st_mode
    is_dir, is_file = stat.S_ISDIR(kind), stat.S_ISREG(kind)
    if mode == 'tree' or (is_file and fnmatch.fnmatch(os.path.basename(shown), pattern)):
        print(shown)
        emitted[0] += 1
        if emitted[0] >= limit:
            return
    if not is_dir or depth >= max_depth:
        return
    try:
        with os.scandir(fd) as entries:
            for entry in entries:
                if visited[0] >= visit_limit or emitted[0] >= limit: break
                try:
                    child = os.open(entry.name, os.O_RDONLY | os.O_NOFOLLOW, dir_fd=fd)
                except OSError:
                    continue
                try: walk(child, shown.rstrip('/') + '/' + entry.name, depth + 1)
                finally: os.close(child)
    except OSError:
        return
fd = open_beneath(root, rel)
try: walk(fd, display, 0)
finally: os.close(fd)
"#;

pub(super) const REMOTE_READ_SCRIPT: &str = r#"import json, os, stat, sys
mode, root, rel, cap = sys.argv[1], sys.argv[2], sys.argv[3], int(sys.argv[4])
fd = os.open('/', os.O_RDONLY | os.O_DIRECTORY)
try:
    for part in [p for p in root.split('/') if p] + [p for p in rel.split('/') if p]:
        nxt = os.open(part, os.O_RDONLY | os.O_NOFOLLOW, dir_fd=fd)
        os.close(fd); fd = nxt
    meta = os.fstat(fd)
    if mode == 'peek':
        if stat.S_ISDIR(meta.st_mode):
            print(json.dumps({'kind':'directory','entries':os.listdir(fd)[:200],'size':meta.st_size}))
        elif stat.S_ISREG(meta.st_mode):
            data = os.read(fd, cap + 1)
            print(json.dumps({'kind':'file','content':data[:cap].decode('utf-8','replace'),'truncated':len(data)>cap,'size':meta.st_size}))
        else: raise RuntimeError('unsupported file type')
    else:
        if not stat.S_ISREG(meta.st_mode): raise RuntimeError('not a regular file')
        data = os.read(fd, cap + 1)
        if len(data) > cap: raise RuntimeError('file exceeds byte limit')
        sys.stdout.write(data.decode('utf-8','replace'))
finally:
    os.close(fd)
"#;

// ─── find ────────────────────────────────────────────────────────────────────

/// Find files on `host` under `path` matching `pattern`.
///
/// `pattern` is passed as the `-name` argument to `find` — it must not start
/// with `-` (guards against option injection).
pub async fn find(
    host: &HostConfig,
    executor: &dyn SshExecutor,
    path: &str,
    pattern: &str,
    depth: Option<u8>,
    limit: Option<u32>,
) -> Result<Value> {
    validate_scout_read_path(host, path)?;

    // Pattern guard (S-M2): reject leading `-` to prevent option injection,
    // NUL bytes (which would truncate the argv string), and over-length values.
    if pattern.starts_with('-') {
        bail!("find pattern must not start with `-`");
    }
    if pattern.contains('\0') {
        bail!("find pattern must not contain NUL bytes");
    }
    if pattern.len() > 256 {
        bail!("find pattern too long: {} chars (max 256)", pattern.len());
    }

    let depth_str = depth
        .map(|d| d.clamp(1, 20).to_string())
        .unwrap_or_else(|| "10".to_owned());
    let limit = (limit.unwrap_or(WALK_MAX_RESULTS as u32) as usize).clamp(1, WALK_MAX_RESULTS);

    let (files, truncated): (Vec<String>, bool) = if is_local_host(host) {
        let host = host.clone();
        let root = path.to_owned();
        let pattern = pattern.to_owned();
        let walk = tokio::task::spawn_blocking(move || {
            bounded_local_find(&host, &root, &pattern, depth_str, limit)
        })
        .await??;
        (walk.items, walk.truncated)
    } else {
        let (root, relative) = crate::secure_path::root_and_relative(host, path)?;
        let limit_arg = limit.to_string();
        let visit_limit = WALK_MAX_VISITED.to_string();
        let remote_args = vec![
            "-c",
            REMOTE_WALK_SCRIPT,
            "find",
            root.as_str(),
            relative.as_str(),
            path,
            pattern,
            depth_str.as_str(),
            limit_arg.as_str(),
            visit_limit.as_str(),
        ];
        let out = RemoteExec { executor, host }
            .run("python3", &remote_args)
            .await?;
        let files = out
            .stdout
            .lines()
            .filter(|line| !line.is_empty())
            .take(limit)
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();
        let truncated = files.len() >= limit;
        (files, truncated)
    };

    Ok(json!({
        "host": host.name,
        "path": path,
        "pattern": pattern,
        "count": files.len(),
        "files": files,
        "truncated": truncated,
    }))
}

pub(super) struct BoundedWalk {
    pub(super) items: Vec<String>,
    pub(super) truncated: bool,
}

pub(super) fn bounded_local_walk(
    host: &HostConfig,
    root: &str,
    max_depth: u8,
) -> Result<BoundedWalk> {
    let mut walk = BoundedWalk {
        items: Vec::new(),
        truncated: false,
    };
    let mut visited = 0;
    visit_local_tree(
        host,
        std::path::Path::new(root),
        0,
        max_depth,
        &mut visited,
        &mut walk,
    )?;
    Ok(walk)
}

fn bounded_local_find(
    host: &HostConfig,
    root: &str,
    pattern: &str,
    depth: String,
    limit: usize,
) -> Result<BoundedWalk> {
    let max_depth = depth.parse::<u8>().unwrap_or(10);
    let context = FindWalkContext {
        host,
        pattern,
        max_depth,
        limit,
    };
    let mut walk = BoundedWalk {
        items: Vec::new(),
        truncated: false,
    };
    let mut visited = 0;
    visit_local_find(
        &context,
        std::path::Path::new(root),
        0,
        &mut visited,
        &mut walk,
    )?;
    Ok(walk)
}

fn visit_local_tree(
    host: &HostConfig,
    path: &std::path::Path,
    depth: u8,
    max_depth: u8,
    visited: &mut usize,
    walk: &mut BoundedWalk,
) -> Result<()> {
    if *visited >= WALK_MAX_VISITED || walk.items.len() >= WALK_MAX_RESULTS {
        walk.truncated = true;
        return Ok(());
    }
    *visited += 1;
    walk.items.push(path.to_string_lossy().into_owned());
    let bound = crate::secure_path::bind_read_path(host, &path.to_string_lossy())?;
    if depth >= max_depth || !bound.file().metadata()?.is_dir() {
        return Ok(());
    }
    for entry in std::fs::read_dir(bound.proc_path())?.filter_map(Result::ok) {
        visit_local_tree(
            host,
            &path.join(entry.file_name()),
            depth + 1,
            max_depth,
            visited,
            walk,
        )?;
        if walk.truncated {
            break;
        }
    }
    Ok(())
}

struct FindWalkContext<'a> {
    host: &'a HostConfig,
    pattern: &'a str,
    max_depth: u8,
    limit: usize,
}

fn visit_local_find(
    context: &FindWalkContext<'_>,
    path: &std::path::Path,
    depth: u8,
    visited: &mut usize,
    walk: &mut BoundedWalk,
) -> Result<()> {
    if *visited >= WALK_MAX_VISITED || walk.items.len() >= context.limit {
        walk.truncated = true;
        return Ok(());
    }
    *visited += 1;
    let bound = crate::secure_path::bind_read_path(context.host, &path.to_string_lossy())?;
    let metadata = bound.file().metadata()?;
    if metadata.is_file()
        && glob_matches(
            context.pattern,
            &path.file_name().unwrap_or_default().to_string_lossy(),
        )
    {
        walk.items.push(path.to_string_lossy().into_owned());
        if walk.items.len() >= context.limit {
            walk.truncated = true;
            return Ok(());
        }
    }
    if depth >= context.max_depth || !metadata.is_dir() {
        return Ok(());
    }
    for entry in std::fs::read_dir(bound.proc_path())?.filter_map(Result::ok) {
        visit_local_find(
            context,
            &path.join(entry.file_name()),
            depth + 1,
            visited,
            walk,
        )?;
        if walk.truncated {
            break;
        }
    }
    Ok(())
}

fn glob_matches(pattern: &str, name: &str) -> bool {
    if pattern == "*" {
        true
    } else if let Some(suffix) = pattern.strip_prefix('*') {
        name.ends_with(suffix)
    } else if let Some(prefix) = pattern.strip_suffix('*') {
        name.starts_with(prefix)
    } else {
        name == pattern
    }
}
