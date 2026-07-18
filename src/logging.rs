//! Dual-output logging — console (colored) + file (JSON).
//!
//! # TEMPLATE: Why dual logging?
//!
//! Two simultaneous log destinations serve different audiences:
//!
//! | Destination | Format     | Audience              | Purpose                    |
//! |-------------|------------|-----------------------|----------------------------|
//! | stderr      | Pretty     | Developer / operator  | Human-readable, colorized  |
//! | File        | JSON lines | Log aggregator / AI   | Machine-parseable, indexed |
//!
//! The console output is optimized for human scanning: compact timestamps,
//! semantic colors, noise suppression. The file output preserves all fields
//! for programmatic analysis (grep, jq, log aggregators, AI agents).
//!
//! # TEMPLATE: Usage in main.rs
//!
//! Replace the existing `tracing_subscriber` setup in `main.rs` with:
//!
//! ```rust,ignore
//! use synapse2::logging;
//!
//! let data_dir = config.data_dir(); // e.g. ~/.synapse2
//! logging::init(&data_dir, "synapse2")?;
//! ```
//!
//! In stdio mode, suppress all logs to avoid polluting the MCP JSON stream:
//!
//! ```rust,ignore
//! if stdio_mode {
//!     // Don't call logging::init() — tracing stays at warn level on stderr only
//!     tracing_subscriber::fmt()
//!         .with_env_filter(EnvFilter::new("warn"))
//!         .with_writer(std::io::stderr)
//!         .init();
//! } else {
//!     logging::init(&data_dir, "synapse2")?;
//! }
//! ```
//!
//! # TEMPLATE: Log file location
//!
//! Logs are written to `{data_dir}/logs/{service}.log`.
//! For the synapse2 service this resolves to `~/.synapse2/logs/synapse2.log`.
//!
//! The file rotates at 10 MiB with three retained archives. This keeps
//! disk usage predictable even for long-running processes.
//! For production deployments that need persistent logs, configure a log
//! aggregator (e.g. Loki, Datadog, CloudWatch) to ship from stderr instead.

pub mod aurora;
pub mod formatter;

use std::io::{IsTerminal, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

use formatter::AuroraFormatter;

/// Detect the desired log format from `LOG_FORMAT` or `RUST_LOG_FORMAT`.
///
/// Returns `true` when the caller should emit JSON (structured) log lines.
/// Either variable may be set to `"json"` (case-insensitive); any other value
/// (or absence of both variables) selects the default human-readable format.
///
/// # Environment variables
///
/// | Variable | Values | Effect |
/// |---|---|---|
/// | `LOG_FORMAT` | `json` | Enable JSON formatter |
/// | `RUST_LOG_FORMAT` | `json` | Enable JSON formatter (alternative name) |
///
/// Both variables are checked; `LOG_FORMAT` takes precedence when both are set.
pub fn json_format_requested() -> bool {
    for var in ["LOG_FORMAT", "RUST_LOG_FORMAT"] {
        if let Ok(val) = std::env::var(var) {
            return val.trim().eq_ignore_ascii_case("json");
        }
    }
    false
}

/// Initialise dual logging: pretty console (stderr) + JSON file.
///
/// When `LOG_FORMAT=json` or `RUST_LOG_FORMAT=json` is set, the console layer
/// also emits JSON lines instead of the human-readable Aurora format. This is
/// useful in container environments where stdout/stderr is captured by a log
/// aggregator (Loki, Datadog, etc.).
///
/// # Arguments
///
/// - `data_dir` — service data directory (e.g. `~/.synapse2`). Logs go into
///   `{data_dir}/logs/{service_name}.log`.
/// - `service_name` — used as the log file name (e.g. `"synapse2"`).
///
/// # Errors
///
/// Returns an error if the log directory cannot be created or the log file
/// cannot be opened for writing.
///
/// # TEMPLATE: EnvFilter precedence
///
/// Log levels are controlled by `RUST_LOG`. If unset, defaults to `"info"`.
/// Examples:
/// - `RUST_LOG=debug` — show all debug logs
/// - `RUST_LOG=info,rmcp=warn` — info level, suppress rmcp crate noise
/// - `RUST_LOG=synapse2=trace` — trace this crate only
///
/// Both the console and file writers share the same `EnvFilter`, so they
/// always emit the same set of events.
pub fn init(data_dir: &Path, service_name: &str) -> Result<()> {
    let log_dir = data_dir.join("logs");
    std::fs::create_dir_all(&log_dir)
        .with_context(|| format!("failed to create log directory: {}", log_dir.display()))?;

    let log_path = log_dir.join(format!("{service_name}.log"));
    let log_writer = RotatingLogWriter::new(log_path.clone())?;

    let console_ansi = should_colorize();
    let use_json = json_format_requested();

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    // TEMPLATE: Subscriber stack
    //
    // The stack is built as:
    //   registry()          — the base subscriber that stores span data
    //     .with(env_filter) — shared level filter for ALL layers
    //     .with(console)    — pretty or JSON stderr output
    //     .with(file)       — JSON lines file output
    //
    // Both layers share the same filter. To give them independent filters,
    // see `tracing_subscriber::layer::Filtered`.
    //
    // When LOG_FORMAT=json (or RUST_LOG_FORMAT=json) the console layer emits
    // JSON instead of the human-readable Aurora format. The file layer is
    // always JSON regardless of this setting.
    if use_json {
        // JSON stderr + JSON file — useful in container environments.
        tracing_subscriber::registry()
            .with(filter)
            .with(
                tracing_subscriber::fmt::layer()
                    .json()
                    .with_ansi(false)
                    .with_writer(std::io::stderr),
            )
            .with(
                tracing_subscriber::fmt::layer()
                    .json()
                    .with_ansi(false)
                    .with_writer({
                        let writer = log_writer.clone();
                        move || writer.clone()
                    }),
            )
            .init();
    } else {
        // Pretty console (Aurora) + JSON file — default human-readable mode.
        tracing_subscriber::registry()
            .with(filter)
            .with(
                // Console layer: pretty, colored, human-readable
                //
                // TEMPLATE: Console layer configuration
                // - `with_ansi(console_ansi)` — enables ANSI codes only when stderr is a TTY
                //   or FORCE_COLOR is set. The AuroraFormatter reads `writer.has_ansi_escapes()`
                //   to conditionally apply colors.
                // - `with_writer(std::io::stderr)` — logs go to stderr, not stdout.
                //   stdout is reserved for CLI output and MCP JSON streams.
                // - `.event_format(AuroraFormatter)` — our custom formatter (see formatter.rs)
                tracing_subscriber::fmt::layer()
                    .with_ansi(console_ansi)
                    .with_writer(std::io::stderr)
                    .event_format(AuroraFormatter),
            )
            .with(
                // File layer: structured JSON, machine-readable
                //
                // TEMPLATE: File layer configuration
                // - `.json()` — emit one JSON object per log line (NDJSON format)
                // - `.with_ansi(false)` — never emit ANSI codes to the file
                // - `.with_writer(log_file)` — write to the log file we opened above
                //
                // JSON format synapse2:
                // {"timestamp":"2026-05-13T14:32:01.123Z","level":"INFO","fields":{"message":"starting","bind":"0.0.0.0:3000"}}
                tracing_subscriber::fmt::layer()
                    .json()
                    .with_ansi(false)
                    .with_writer({
                        let writer = log_writer.clone();
                        move || writer.clone()
                    }),
            )
            .init();
    }

    tracing::debug!(
        log_file = %log_path.display(),
        ansi = console_ansi,
        json_format = use_json,
        "logging initialised"
    );

    Ok(())
}

// ── Log file rotation ─────────────────────────────────────────────────────────

/// Maximum log file size in bytes before truncation.
///
/// # TEMPLATE: Why 10MB?
///
/// 10MB is large enough to contain several hours of busy server logs at INFO
/// level. Three archives retain recent diagnostics while bounding total usage.
///
/// If you need longer retention, configure log shipping to an external system
/// (Loki, Datadog, etc.) and keep this cap. The file is for local debugging.
const LOG_FILE_MAX_BYTES: u64 = 10 * 1024 * 1024; // 10 MiB
const LOG_FILE_RETENTION: usize = 3;

struct RotatingLogState {
    path: std::path::PathBuf,
    file: Option<std::fs::File>,
    size: u64,
}

#[derive(Clone)]
struct RotatingLogWriter(Arc<Mutex<RotatingLogState>>);

impl RotatingLogWriter {
    fn new(path: std::path::PathBuf) -> Result<Self> {
        let size = path.metadata().map(|meta| meta.len()).unwrap_or(0);
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .with_context(|| format!("failed to open log file: {}", path.display()))?;
        Ok(Self(Arc::new(Mutex::new(RotatingLogState {
            path,
            file: Some(file),
            size,
        }))))
    }
}

impl Write for RotatingLogWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut state = self
            .0
            .lock()
            .map_err(|_| std::io::Error::other("log lock poisoned"))?;
        if state.size.saturating_add(buf.len() as u64) > LOG_FILE_MAX_BYTES {
            rotate_log(&mut state)?;
        }
        let written = state
            .file
            .as_mut()
            .expect("rotating log file is open")
            .write(buf)?;
        state.size = state.size.saturating_add(written as u64);
        Ok(written)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.0
            .lock()
            .map_err(|_| std::io::Error::other("log lock poisoned"))?
            .file
            .as_mut()
            .expect("rotating log file is open")
            .flush()
    }
}

fn rotate_log(state: &mut RotatingLogState) -> std::io::Result<()> {
    if let Some(file) = state.file.take() {
        file.sync_all()?;
    }
    let oldest = state
        .path
        .with_extension(format!("log.{LOG_FILE_RETENTION}"));
    let _ = std::fs::remove_file(oldest);
    for index in (1..LOG_FILE_RETENTION).rev() {
        let from = state.path.with_extension(format!("log.{index}"));
        let to = state.path.with_extension(format!("log.{}", index + 1));
        if from.exists() {
            std::fs::rename(from, to)?;
        }
    }
    if state.path.exists() {
        std::fs::rename(&state.path, state.path.with_extension("log.1"))?;
    }
    state.file = Some(
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&state.path)?,
    );
    state.size = 0;
    eprintln!("WARN  log file reached {LOG_FILE_MAX_BYTES} bytes and was rotated");
    Ok(())
}

// ── Colorization detection ────────────────────────────────────────────────────

/// Determine whether console log output should include ANSI color codes.
///
/// Priority order (highest to lowest):
///
/// 1. `NO_COLOR` env var set → **no color** (https://no-color.org convention)
/// 2. `FORCE_COLOR` env var set → **force color** (useful in Docker/CI)
/// 3. `stderr` is a TTY → **color** (interactive terminal)
/// 4. `stderr` is not a TTY → **no color** (piped/redirected)
///
/// # TEMPLATE: Docker containers
///
/// Docker containers often do NOT have a TTY attached to stderr, which would
/// disable color by rule 4. But `docker compose logs` renders ANSI codes
/// correctly, so operators benefit from colors.
///
/// Set `FORCE_COLOR=1` in your `docker-compose.yml` or Dockerfile:
/// ```yaml
/// environment:
///   FORCE_COLOR: "1"
/// ```
///
/// # TEMPLATE: CI/CD pipelines
///
/// Most CI systems (GitHub Actions, GitLab CI) support ANSI codes.
/// Set `FORCE_COLOR=1` in your CI environment variables to enable color logs.
pub fn should_colorize() -> bool {
    // NO_COLOR takes precedence over everything (https://no-color.org)
    if std::env::var_os("NO_COLOR").is_some() {
        return false;
    }

    // FORCE_COLOR overrides TTY detection (for Docker, CI, etc.)
    if std::env::var_os("FORCE_COLOR").is_some() {
        return true;
    }

    // Fall back to TTY detection
    std::io::stderr().is_terminal()
}

#[cfg(test)]
#[path = "logging_tests.rs"]
mod tests;
