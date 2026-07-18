//! Compose project discovery, lister, and per-host discovery cache.
//!
//! This is the **discovery layer** for `flux.compose`: it finds compose projects
//! on a host (local or remote) and caches the merged view. It does **not** run
//! compose operations (up/down/restart/…) — that is B13, which consumes this
//! module's [`ComposeDiscovery`] engine.
//!
//! Design:
//!
//! - **Discovery sources, then merge.** Two sources feed the project list:
//!   1. `docker compose ls --format json` — authoritative for *running/known*
//!      projects (status, service count).
//!   2. A filesystem `find` over the host's `compose_search_paths` — finds
//!      *stopped* projects with no running containers.
//!
//!   Results merge keyed by project name; the docker-ls entry wins on conflict
//!   (it carries live status), but its empty config path is backfilled from the
//!   filesystem scan when available.
//! - **All commands go over [`SshExecutor`] (B1)** — execvp-style, no shell.
//!   This keeps discovery independent of B2's bollard wiring; the locked
//!   decision permits the SSH/shell path for `docker compose ls`.
//!
//!   LOCALHOST CAVEAT: B1's `SshPool` has no `HostProtocol::Local` branch yet —
//!   it SSHes even to `localhost`. Running discovery against a *local* host
//!   therefore requires an executor with a local-exec branch (or `~/.ssh/config`
//!   set up for loopback). Wiring local routing into the shared executor is a
//!   merge-time concern (see B12 integration notes); B12 takes the executor as
//!   an injected `Arc<dyn SshExecutor>` and does not assume a routing strategy.
//! - **60s TTL cache, per host**, keyed by complete host topology identity.
//!   `refresh(host)` invalidates one host; `refresh(None)` invalidates all.
//! - **Project name** comes from the compose file's top-level `name:` field,
//!   else the parent directory name (matches `docker compose` behavior).
//! - **Search-path validation** is string-only (absolute, no `..`, safe chars).
//!   It deliberately does NOT use [`crate::synapse::validate_safe_path`], whose
//!   `symlink_metadata` check runs against the *local* FS and is meaningless
//!   (and false-positive-prone) for remote search roots.

use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use std::time::Duration;

use anyhow::{Result, bail};
use serde::Serialize;

use crate::cache::{Cache, MemoryCache};
use crate::ssh::SshExecutor;
use crate::synapse::HostConfig;

#[cfg(test)]
#[path = "compose_tests.rs"]
mod tests;

/// Default Unraid-centric search roots, applied when a host sets no
/// `composeSearchPaths`. Matches synapse-mcp `DEFAULT_COMPOSE_SEARCH_PATHS`.
pub const DEFAULT_COMPOSE_SEARCH_PATHS: &[&str] =
    &["/compose", "/mnt/cache/compose", "/mnt/cache/code"];

/// Compose file names recognized by the filesystem scan.
pub const COMPOSE_FILE_NAMES: &[&str] = &[
    "docker-compose.yml",
    "docker-compose.yaml",
    "compose.yml",
    "compose.yaml",
];

/// Maximum directory depth for the filesystem `find` walk.
pub const MAX_SCAN_DEPTH: u32 = 3;

/// Default discovery cache TTL (matches synapse-mcp's 60s).
pub const DEFAULT_CACHE_TTL: Duration = Duration::from_secs(60);

/// Fixed, argv-only batch parser used to resolve every discovered compose name
/// in one host round trip. It reads at most 64 KiB from each file.
const COMPOSE_NAME_BATCH_SCRIPT: &str = r#"import json, sys
results = {}
for path in sys.argv[1:]:
    name = None
    try:
        with open(path, 'r', encoding='utf-8', errors='replace') as handle:
            content = handle.read(65536)
        for line in content.splitlines():
            if line[:1].isspace() or not line.startswith('name:'):
                continue
            value = line[5:].strip()
            if value[:1] in ('\"', \"'\"):
                quote = value[0]
                end = value.find(quote, 1)
                value = value[1:end] if end >= 0 else value[1:]
            else:
                value = value.split('#', 1)[0].strip()
            name = value or None
            break
    except OSError:
        pass
    if name is not None:
        results[path] = name
print(json.dumps(results, separators=(',', ':')))
"#;

/// Where a discovered project's information came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DiscoveredFrom {
    /// Found via `docker compose ls` (running/known to the daemon).
    DockerLs,
    /// Found via a filesystem scan (may be stopped).
    Scan,
}

/// A single discovered compose project. This is the hand-off type B13 consumes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ComposeProject {
    /// Project name (explicit `name:` field, else parent directory name).
    pub name: String,
    /// Absolute paths to the project's compose file(s). May be empty when a
    /// running project reports no config files.
    pub config_files: Vec<String>,
    /// Raw `docker compose ls` status string (empty for scan-only projects).
    pub status: String,
    /// Total container/service count parsed from the status (0 when unknown).
    pub service_count: u32,
    /// Which discovery source produced this entry.
    pub discovered_from: DiscoveredFrom,
}

impl ComposeProject {
    /// Primary compose file path, if any (the first config file).
    pub fn primary_config_file(&self) -> Option<&str> {
        self.config_files.first().map(String::as_str)
    }
}

/// Validate a single compose search path (string-only; remote-safe).
///
/// SECURITY (security-sentinel, LOW): a relative or `..`-bearing path would let
/// an attacker plant a malicious compose file relative to CWD. Require an
/// absolute path, reject `..` components, and restrict to safe characters.
///
/// This intentionally does NOT call [`crate::synapse::validate_safe_path`]:
/// that validator additionally calls `symlink_metadata` against the *local*
/// filesystem, which is meaningless for remote search roots and would falsely
/// reject paths that don't exist locally or are local symlinks.
pub fn validate_search_path(path: &str) -> Result<()> {
    if path.is_empty() {
        bail!("compose search path must not be empty");
    }
    if !path.starts_with('/') {
        bail!("compose search path must be absolute: {path}");
    }
    if path.split('/').any(|part| part == "..") {
        bail!("compose search path may not contain '..': {path}");
    }
    if !path
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '.' | '_' | '-'))
    {
        bail!("compose search path contains unsafe characters: {path}");
    }
    Ok(())
}

/// Resolve the effective search paths for a host: its configured
/// `composeSearchPaths`, or the defaults when none are set. Each path is
/// validated; invalid paths are dropped (with a warning) rather than failing
/// the whole scan.
pub fn effective_search_paths(host: &HostConfig) -> Vec<String> {
    let configured: Vec<String> = if host.compose_search_paths.is_empty() {
        DEFAULT_COMPOSE_SEARCH_PATHS
            .iter()
            .map(|s| s.to_string())
            .collect()
    } else {
        host.compose_search_paths.clone()
    };

    let mut out = Vec::with_capacity(configured.len());
    for path in configured {
        match validate_search_path(&path) {
            Ok(()) => out.push(path),
            Err(e) => tracing::warn!(host = %host.name, "skipping compose search path: {e}"),
        }
    }
    out
}

/// Parse the total service/container count from a `docker compose ls` status.
///
/// Docker encodes counts as `running(5)`, `running(2), exited(1)`, etc. Sum all
/// `(N)` occurrences; returns 0 when no numeric count is present.
pub fn parse_service_count(status: &str) -> u32 {
    let mut total: u32 = 0;
    let bytes = status.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'(' {
            let mut j = i + 1;
            let mut n: u32 = 0;
            let mut saw_digit = false;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                n = n
                    .saturating_mul(10)
                    .saturating_add((bytes[j] - b'0') as u32);
                saw_digit = true;
                j += 1;
            }
            if saw_digit && j < bytes.len() && bytes[j] == b')' {
                total = total.saturating_add(n);
                i = j + 1;
                continue;
            }
        }
        i += 1;
    }
    total
}

/// Extract the parent-directory name from a compose file path. Used as the
/// project name when the file has no explicit top-level `name:` field.
pub fn project_name_from_path(file_path: &str) -> String {
    let parts: Vec<&str> = file_path.split('/').filter(|p| !p.is_empty()).collect();
    if parts.len() < 2 {
        return String::new();
    }
    parts[parts.len() - 2].to_string()
}

/// The compose discovery engine: filesystem scan + `docker compose ls` lister +
/// a per-host TTL cache of the merged project list.
///
/// Clone-cheap: the cache lives behind the engine's own `Arc` (held by the
/// service), so all callers share one cache. Construct once and wrap in `Arc`.
pub struct ComposeDiscovery {
    ssh: Arc<dyn SshExecutor>,
    /// Cache keyed by complete topology/discovery identity and refresh generation.
    cache: MemoryCache<String, Vec<ComposeProject>>,
    in_flight: dashmap::DashMap<String, Arc<tokio::sync::Mutex<()>>>,
    refresh_all_generation: AtomicU64,
    refresh_host_generations: dashmap::DashMap<String, u64>,
}

impl ComposeDiscovery {
    /// Build a discovery engine over the given SSH executor with the default
    /// 60s cache TTL.
    pub fn new(ssh: Arc<dyn SshExecutor>) -> Self {
        Self::with_ttl(ssh, DEFAULT_CACHE_TTL)
    }

    /// Build a discovery engine with a custom cache TTL (per-host configurable
    /// TTL, per the locked decision).
    pub fn with_ttl(ssh: Arc<dyn SshExecutor>, ttl: Duration) -> Self {
        Self {
            ssh,
            cache: MemoryCache::with_ttl(ttl),
            in_flight: dashmap::DashMap::new(),
            refresh_all_generation: AtomicU64::new(0),
            refresh_host_generations: dashmap::DashMap::new(),
        }
    }

    /// List all compose projects on `host`, merged from `docker compose ls` and
    /// a filesystem scan. Cache-aware: a fresh entry short-circuits discovery.
    ///
    /// Validation requirement: `flux compose list` returns projects from both
    /// the filesystem scan and active `docker compose ls`.
    pub async fn list(&self, host: &HostConfig) -> Result<Vec<ComposeProject>> {
        let generation = self.refresh_generation(&host.name);
        let cache_key = compose_cache_key(host, generation);
        if let Some(cached) = self.cache.get(&cache_key)
            && self.refresh_generation(&host.name) == generation
        {
            return Ok(cached);
        }
        let lock = self
            .in_flight
            .entry(cache_key.clone())
            .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
            .clone();
        let _guard = lock.lock().await;
        let result = if let Some(cached) = self.cache.get(&cache_key)
            && self.refresh_generation(&host.name) == generation
        {
            Ok(cached)
        } else {
            match self.discover(host).await {
                Ok(merged) => {
                    if self.refresh_generation(&host.name) == generation {
                        self.cache.set(cache_key.clone(), merged.clone());
                    }
                    Ok(merged)
                }
                Err(error) => Err(error),
            }
        };
        drop(_guard);
        // Keep a key while callers are queued, then remove it once only the map
        // and this function own the Arc. `remove_if` locks the shard while the
        // strong-count predicate runs, so a racing lookup cannot be orphaned.
        self.in_flight.remove_if(&cache_key, |_, current| {
            Arc::ptr_eq(current, &lock) && Arc::strong_count(current) == 2
        });
        result
    }

    /// Invalidate the discovery cache. With a host name, clears only that host;
    /// with `None`, clears all hosts. The next `list()` re-scans.
    pub fn refresh(&self, host_name: Option<&str>) {
        match host_name {
            Some(name) => {
                *self
                    .refresh_host_generations
                    .entry(name.to_owned())
                    .or_insert(0) += 1;
            }
            None => {
                self.refresh_all_generation.fetch_add(1, Ordering::AcqRel);
                self.cache.invalidate_all();
            }
        }
    }

    fn refresh_generation(&self, host_name: &str) -> (u64, u64) {
        (
            self.refresh_all_generation.load(Ordering::Acquire),
            *self
                .refresh_host_generations
                .entry(host_name.to_owned())
                .or_insert(0),
        )
    }

    /// Run discovery (no cache): merge `docker compose ls` and filesystem scan.
    async fn discover(&self, host: &HostConfig) -> Result<Vec<ComposeProject>> {
        // Source 1: docker compose ls (running/known projects). Failure here is
        // non-fatal — fall back to filesystem-only results.
        let active = match self.list_active(host).await {
            Ok(projects) => projects,
            Err(e) => {
                tracing::warn!(host = %host.name, "docker compose ls failed: {e}");
                Vec::new()
            }
        };

        // Source 2: filesystem scan (finds stopped projects).
        let scanned = self.scan_filesystem(host).await?;

        Ok(merge_projects(active, scanned))
    }

    /// Run `docker compose ls --format json` on the host and parse the result.
    async fn list_active(&self, host: &HostConfig) -> Result<Vec<ComposeProject>> {
        let output = self
            .ssh
            .exec(host, "docker", &["compose", "ls", "--format", "json"])
            .await?;
        parse_compose_ls(&output.stdout)
    }

    /// Scan the host's search paths for compose files and resolve their names.
    async fn scan_filesystem(&self, host: &HostConfig) -> Result<Vec<ComposeProject>> {
        let search_paths = effective_search_paths(host);
        if search_paths.is_empty() {
            return Ok(Vec::new());
        }

        let files = self.find_compose_files(host, &search_paths).await?;

        let names = self.read_compose_names(host, &files).await;
        Ok(files
            .into_iter()
            .filter_map(|file| {
                let name = names
                    .get(&file)
                    .and_then(|name| name.clone())
                    .unwrap_or_else(|| project_name_from_path(&file));
                (!name.is_empty()).then(|| ComposeProject {
                    name,
                    config_files: vec![file],
                    status: String::new(),
                    service_count: 0,
                    discovered_from: DiscoveredFrom::Scan,
                })
            })
            .collect())
    }

    /// Build and run the `find` command, returning compose file paths.
    ///
    /// `find` exits nonzero when a search path is missing but still prints
    /// matches under paths that exist, so stdout is parsed regardless of exit
    /// code — a missing path yields an empty list, never an error.
    async fn find_compose_files(
        &self,
        host: &HostConfig,
        search_paths: &[String],
    ) -> Result<Vec<String>> {
        let mut args: Vec<&str> = Vec::new();
        for p in search_paths {
            args.push(p.as_str());
        }
        args.push("-maxdepth");
        let depth = MAX_SCAN_DEPTH.to_string();
        args.push(&depth);
        args.push("-type");
        args.push("f");
        args.push("(");
        for (i, name) in COMPOSE_FILE_NAMES.iter().enumerate() {
            if i > 0 {
                args.push("-o");
            }
            args.push("-name");
            args.push(name);
        }
        args.push(")");
        args.push("-print");

        let output = self.ssh.exec(host, "find", &args).await?;
        Ok(parse_find_output(&output.stdout))
    }

    /// Resolve all explicit top-level compose names in one remote process.
    async fn read_compose_names(
        &self,
        host: &HostConfig,
        files: &[String],
    ) -> std::collections::HashMap<String, Option<String>> {
        if files.is_empty() {
            return std::collections::HashMap::new();
        }
        let mut args = Vec::with_capacity(files.len() + 2);
        args.push("-c");
        args.push(COMPOSE_NAME_BATCH_SCRIPT);
        args.extend(files.iter().map(String::as_str));
        let Ok(output) = self.ssh.exec(host, "python3", &args).await else {
            return std::collections::HashMap::new();
        };
        if !output.success() {
            return std::collections::HashMap::new();
        }
        serde_json::from_str(&output.stdout).unwrap_or_default()
    }

    #[cfg(test)]
    fn in_flight_len(&self) -> usize {
        self.in_flight.len()
    }
}

fn compose_cache_key(host: &HostConfig, generation: (u64, u64)) -> String {
    let search_paths = effective_search_paths(host).join("\u{1f}");
    format!(
        "{}|compose-search={search_paths}|refresh={}:{}",
        host.connection_key(),
        generation.0,
        generation.1
    )
}

/// Parse `find -print` stdout into a deduplicated list of file paths.
pub fn parse_find_output(stdout: &str) -> Vec<String> {
    let mut seen = std::collections::BTreeSet::new();
    let mut out = Vec::new();
    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if seen.insert(line.to_string()) {
            out.push(line.to_string());
        }
    }
    out
}

/// Parse `docker compose ls --format json` stdout into [`ComposeProject`]s.
///
/// Empty/blank stdout yields an empty list. Each entry carries `Name`,
/// `Status`, and `ConfigFiles` (comma-separated).
pub fn parse_compose_ls(stdout: &str) -> Result<Vec<ComposeProject>> {
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    #[derive(serde::Deserialize)]
    struct Raw {
        #[serde(rename = "Name")]
        name: String,
        #[serde(rename = "Status", default)]
        status: String,
        #[serde(rename = "ConfigFiles", default)]
        config_files: String,
    }

    let raw: Vec<Raw> = serde_json::from_str(trimmed)
        .map_err(|e| anyhow::anyhow!("unexpected `docker compose ls` output: {e}"))?;

    Ok(raw
        .into_iter()
        .map(|r| {
            let config_files = if r.config_files.trim().is_empty() {
                Vec::new()
            } else {
                r.config_files
                    .split(',')
                    .map(|f| f.trim().to_string())
                    .filter(|f| !f.is_empty())
                    .collect()
            };
            let service_count = parse_service_count(&r.status);
            ComposeProject {
                name: r.name,
                config_files,
                status: r.status,
                service_count,
                discovered_from: DiscoveredFrom::DockerLs,
            }
        })
        .collect())
}

/// Merge active (`docker compose ls`) and scanned (filesystem) projects, keyed
/// by name. The docker-ls entry wins on conflict (it carries live status); its
/// empty `config_files` is backfilled from the scan when available. Output is
/// sorted by name for deterministic results.
pub fn merge_projects(
    active: Vec<ComposeProject>,
    scanned: Vec<ComposeProject>,
) -> Vec<ComposeProject> {
    use std::collections::BTreeMap;
    let mut by_name: BTreeMap<String, ComposeProject> = BTreeMap::new();

    // Filesystem scan first (lower precedence).
    for p in scanned {
        by_name.insert(p.name.clone(), p);
    }

    // docker-ls wins; backfill its config files from a prior scan entry.
    for mut p in active {
        if p.config_files.is_empty()
            && let Some(prev) = by_name.get(&p.name)
            && !prev.config_files.is_empty()
        {
            p.config_files = prev.config_files.clone();
        }
        by_name.insert(p.name.clone(), p);
    }

    by_name.into_values().collect()
}
