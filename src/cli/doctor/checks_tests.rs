//! Unit tests for src/cli/doctor/checks.rs
//!
//! Declared in checks.rs as:
//! ```rust
//! #[cfg(test)]
//! #[path = "checks_tests.rs"]
//! mod tests;
//! ```
//!
//! Tests cover the pure and near-pure check functions. Filesystem-heavy checks
//! are covered with minimal scaffolding.

use super::*;
use crate::config::McpConfig;

// ── check_binary_in_path ─────────────────────────────────────────────────────

#[test]
fn binary_in_path_passes_for_sh() {
    // /bin/sh or /usr/bin/sh is on PATH in any POSIX system.
    let check = check_binary_in_path("sh");
    assert!(check.ok, "sh should be found in PATH");
    assert_eq!(check.category, "config");
}

#[test]
fn binary_in_path_fails_for_nonexistent() {
    let check = check_binary_in_path("this-binary-definitely-does-not-exist-rmcp");
    assert!(!check.ok, "unknown binary should fail");
    let hint = check.hint.unwrap();
    assert!(hint.contains("PATH"), "hint should mention PATH");
}

// ── check_port_available ─────────────────────────────────────────────────────

#[test]
fn port_available_passes_for_free_port() {
    use std::net::TcpListener;
    // Bind to port 0 to get an OS-assigned ephemeral port, then drop the
    // listener so the port is free before calling check_port_available.
    let listener = TcpListener::bind("127.0.0.1:0").expect("should bind to an ephemeral port");
    let port = listener.local_addr().unwrap().port();
    drop(listener); // release the port before the check

    let check = check_port_available("127.0.0.1", port);
    assert_eq!(check.category, "server");
    assert!(check.ok, "a free port should pass the availability check");
}

#[test]
fn port_available_fails_when_already_bound() {
    use std::net::TcpListener;
    let listener = TcpListener::bind("127.0.0.1:0").expect("should bind to an ephemeral port");
    let port = listener.local_addr().unwrap().port();

    let check = check_port_available("127.0.0.1", port);
    assert!(!check.ok, "port in use should fail");
    assert!(
        check.hint.unwrap().contains(&port.to_string()),
        "hint should name the port"
    );
}

// ── check_config_file ────────────────────────────────────────────────────────

#[test]
fn config_file_passes_when_present() {
    let dir = tempfile::tempdir().expect("should create temp dir");
    let config_path = dir.path().join("config.toml");
    std::fs::write(&config_path, b"[mcp]\nport = 3000\n").unwrap();

    let check = check_config_file(dir.path());
    assert!(check.ok);
    assert!(check.value.unwrap().contains("config.toml"));
}

#[test]
fn config_file_passes_gracefully_when_absent() {
    let dir = tempfile::tempdir().expect("should create temp dir");
    let check = check_config_file(dir.path());
    // Missing config.toml is a soft pass (env vars cover it).
    assert!(check.ok, "missing config.toml should not hard-fail");
    assert!(
        check.value.unwrap().contains("not found"),
        "value should note the file is missing"
    );
}

// ── check_dir_writable ───────────────────────────────────────────────────────

#[test]
fn dir_writable_passes_for_writable_dir() {
    let dir = tempfile::tempdir().expect("should create temp dir");
    let check = check_dir_writable("Test dir", dir.path());
    assert!(check.ok);
    assert!(check.value.unwrap().contains("writable"));
}

#[cfg(unix)]
#[test]
fn dir_writable_does_not_recurse_into_symlinked_children() {
    let dir = tempfile::tempdir().expect("should create temp dir");
    std::os::unix::fs::symlink(dir.path(), dir.path().join("loop")).unwrap();

    let check = check_dir_writable("Test dir", dir.path());
    assert!(
        check.ok,
        "writability check should not traverse symlinked children"
    );
}

fn auth_config(host: &str) -> Config {
    Config {
        mcp: McpConfig {
            host: host.into(),
            ..McpConfig::default()
        },
    }
}

#[test]
fn auth_config_passes_loopback_no_auth() {
    let mut config = auth_config("127.0.0.1");
    config.mcp.no_auth = true;

    let check = check_auth_config(&config);

    assert!(check.ok);
    assert!(check.value.unwrap().contains("loopback"));
}

#[test]
fn auth_config_rejects_non_loopback_without_auth() {
    let config = auth_config("0.0.0.0");

    let check = check_auth_config(&config);

    assert!(!check.ok);
    assert!(check.hint.unwrap().contains("SYNAPSE_MCP_TOKEN"));
}
