//! Unit tests for scout filesystem operations (peek/find/delta).

use super::*;
use crate::ssh::{CommandOutput, SshExecutor};
use crate::synapse::HostConfig;
use async_trait::async_trait;
use std::sync::{Arc, Mutex};

type RecordedSshCalls = Arc<Mutex<Vec<(String, Vec<String>)>>>;

// ─── validate_safe_path tests (security-critical) ────────────────────────────

#[test]
fn peek_rejects_relative_path() {
    // Must be an async test since peek is async, but path validation happens
    // synchronously — we can test validate_safe_path directly.
    let result = crate::synapse::validate_safe_path("relative/path");
    assert!(result.is_err(), "relative path must be rejected");
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("absolute"),
        "error must mention absolute: {msg}"
    );
}

#[test]
fn peek_rejects_dotdot() {
    let result = crate::synapse::validate_safe_path("/tmp/../etc/passwd");
    assert!(result.is_err(), "path with .. must be rejected");
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("traversal") || msg.contains(".."), "{msg}");
}

#[test]
fn peek_rejects_empty_path() {
    let result = crate::synapse::validate_safe_path("");
    assert!(result.is_err(), "empty path must be rejected");
}

#[tokio::test]
async fn peek_local_file_reads_only_preview_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("large.txt");
    std::fs::write(&path, "x".repeat(PEEK_MAX_CONTENT_BYTES + 256)).unwrap();

    let mut host = HostConfig::local();
    host.scout_read_roots = vec![dir.path().to_string_lossy().into_owned()];

    let result = super::peek(&host, &NoopExec, path.to_str().unwrap(), false, 3)
        .await
        .unwrap();

    assert_eq!(result["kind"], "file");
    assert_eq!(
        result["content"].as_str().unwrap().len(),
        PEEK_MAX_CONTENT_BYTES
    );
    assert!(result["truncated"].as_bool().unwrap());
    assert_eq!(
        result["max_content_bytes"].as_u64().unwrap(),
        PEEK_MAX_CONTENT_BYTES as u64
    );
}

#[tokio::test]
async fn peek_remote_file_uses_bounded_head_read() {
    #[derive(Clone, Default)]
    struct RecordingExec {
        calls: RecordedSshCalls,
    }

    #[async_trait]
    impl SshExecutor for RecordingExec {
        async fn exec(
            &self,
            _: &HostConfig,
            program: &str,
            args: &[&str],
        ) -> anyhow::Result<CommandOutput> {
            self.calls.lock().unwrap().push((
                program.to_owned(),
                args.iter().map(|arg| (*arg).to_owned()).collect(),
            ));
            match program {
                "python3" => Ok(CommandOutput {
                    stdout: serde_json::json!({
                        "kind": "file",
                        "content": "y".repeat(PEEK_MAX_CONTENT_BYTES),
                        "truncated": true,
                        "size": PEEK_MAX_CONTENT_BYTES + 500,
                    })
                    .to_string(),
                    stderr: String::new(),
                    exit_code: Some(0),
                }),
                other => anyhow::bail!("unexpected program: {other}"),
            }
        }
    }

    let exec = RecordingExec::default();
    let mut host = HostConfig::local();
    host.name = "remote".into();
    host.host = "remote.example".into();
    host.protocol = crate::synapse::HostProtocol::Ssh;
    host.scout_read_roots = vec!["/tmp".into()];

    let result = super::peek(&host, &exec, "/tmp/large.txt", false, 3)
        .await
        .unwrap();

    assert_eq!(
        result["content"].as_str().unwrap().len(),
        PEEK_MAX_CONTENT_BYTES
    );
    assert!(result["truncated"].as_bool().unwrap());

    let calls = exec.calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "python3");
    assert!(calls[0].1[1].contains("os.O_NOFOLLOW"));
    assert_eq!(calls[0].1[2], "peek");
}

// ─── compute_diff tests ───────────────────────────────────────────────────────

#[test]
fn diff_identical_files_is_empty() {
    let d = compute_diff("hello\nworld\n", "hello\nworld\n", "a", "b");
    assert!(d.is_empty(), "identical files should produce empty diff");
}

#[test]
fn diff_different_files_non_empty() {
    let d = compute_diff("hello\n", "world\n", "a", "b");
    assert!(!d.is_empty(), "different files should produce a diff");
    assert!(d.contains("--- a"), "diff should contain source label");
    assert!(d.contains("+++ b"), "diff should contain target label");
}

// ─── delta content limit ──────────────────────────────────────────────────────

#[tokio::test]
async fn delta_rejects_content_over_1mb() {
    // Content size check fires BEFORE any IO (validate_safe_path runs first,
    // then the content-size guard, then the source-file read).
    // The test verifies the size guard fires before the IO path.
    use crate::ssh::{CommandOutput, SshExecutor};
    use async_trait::async_trait;

    struct EchoExec;
    #[async_trait]
    impl SshExecutor for EchoExec {
        async fn exec(
            &self,
            _host: &HostConfig,
            _program: &str,
            _args: &[&str],
        ) -> anyhow::Result<CommandOutput> {
            // If this is called the test has failed: content check should fire first.
            panic!("IO should not be reached when content exceeds 1 MB");
        }
    }

    let host = HostConfig::local();
    let big_content = "x".repeat(DELTA_MAX_CONTENT_BYTES + 1);

    // Use a syntactically valid absolute path. The content-size check runs
    // before read_remote_file, so the source need not exist.
    let result = delta(
        &host,
        &EchoExec,
        "/tmp/synapse2_test_source",
        None,
        None,
        Some(&big_content),
    )
    .await;

    assert!(result.is_err(), "delta must reject content > 1 MB");
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("1 MB") || msg.contains("content"),
        "error must mention content limit: {msg}"
    );
}

// ─── find pattern guard (S-M2) ───────────────────────────────────────────────

#[test]
fn find_rejects_leading_dash_pattern() {
    // Validate pattern rejection (not async — the check is synchronous inside).
    // We use a runtime to call the async function.
    let rt = tokio::runtime::Runtime::new().unwrap();
    let host = HostConfig::local();

    let result = rt.block_on(super::find(
        &host,
        &NoopExec,
        "/tmp",
        "-exec rm -rf",
        None,
        None,
    ));
    assert!(result.is_err(), "leading dash pattern must be rejected");
}

#[test]
fn find_rejects_nul_byte_in_pattern() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut host = HostConfig::local();
    host.scout_read_roots = vec!["/tmp".into()];

    let result = rt.block_on(super::find(
        &host,
        &NoopExec,
        "/tmp",
        "*.txt\0evil",
        None,
        None,
    ));
    assert!(result.is_err(), "NUL byte in pattern must be rejected");
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("NUL"), "error must mention NUL: {msg}");
}

#[test]
fn find_rejects_over_length_pattern() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut host = HostConfig::local();
    host.scout_read_roots = vec!["/tmp".into()];

    let long_pattern = "a".repeat(257);
    let result = rt.block_on(super::find(
        &host,
        &NoopExec,
        "/tmp",
        &long_pattern,
        None,
        None,
    ));
    assert!(result.is_err(), "over-length pattern must be rejected");
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("too long") || msg.contains("256"),
        "error must mention length: {msg}"
    );
}

#[tokio::test]
async fn remote_find_uses_fixed_bounded_walker_and_argv_limit() {
    #[derive(Default)]
    struct RecordingExec {
        calls: RecordedSshCalls,
    }
    #[async_trait]
    impl SshExecutor for RecordingExec {
        async fn exec(
            &self,
            _: &HostConfig,
            program: &str,
            args: &[&str],
        ) -> anyhow::Result<CommandOutput> {
            self.calls.lock().unwrap().push((
                program.to_owned(),
                args.iter().map(|arg| (*arg).to_owned()).collect(),
            ));
            match program {
                "realpath" => Ok(CommandOutput {
                    stdout: "/tmp/root\n".into(),
                    stderr: String::new(),
                    exit_code: Some(0),
                }),
                "python3" => Ok(CommandOutput {
                    stdout: "/tmp/root/a.log\n/tmp/root/b.log\n".into(),
                    stderr: String::new(),
                    exit_code: Some(0),
                }),
                other => anyhow::bail!("unexpected program: {other}"),
            }
        }
    }

    let exec = RecordingExec::default();
    let mut host = HostConfig::local();
    host.name = "remote".into();
    host.host = "remote.example".into();
    host.protocol = crate::synapse::HostProtocol::Ssh;
    host.scout_read_roots = vec!["/tmp".into()];
    let result = find(&host, &exec, "/tmp/root", "*.log", Some(4), Some(2))
        .await
        .unwrap();
    assert_eq!(result["count"], 2);
    let calls = exec.calls.lock().unwrap();
    let walker = calls.iter().find(|call| call.0 == "python3").unwrap();
    assert_eq!(walker.1[0], "-c");
    assert_eq!(walker.1[2], "find");
    assert_eq!(&walker.1[walker.1.len() - 2], "2");
    assert_eq!(walker.1.last().unwrap(), "10000");
    assert!(walker.1[1].contains("os.scandir"));
}

#[tokio::test]
async fn local_tree_and_find_enforce_hard_result_caps() {
    let dir = tempfile::tempdir().unwrap();
    for index in 0..(WALK_MAX_RESULTS + 25) {
        std::fs::write(dir.path().join(format!("{index}.log")), "x").unwrap();
    }
    let mut host = HostConfig::local();
    host.scout_read_roots = vec![dir.path().to_string_lossy().into_owned()];
    let root = dir.path().to_str().unwrap();

    let tree = peek(&host, &NoopExec, root, true, 1).await.unwrap();
    assert_eq!(
        tree["tree"].as_str().unwrap().lines().count(),
        WALK_MAX_RESULTS
    );
    assert_eq!(tree["truncated"], true);

    let found = find(&host, &NoopExec, root, "*.log", Some(1), Some(u32::MAX))
        .await
        .unwrap();
    assert_eq!(found["count"], WALK_MAX_RESULTS);
    assert_eq!(found["truncated"], true);
}

// ─── remote symlink guard (S-M1) ─────────────────────────────────────────────

#[tokio::test]
async fn peek_remote_rejects_symlink_path() {
    /// Mock executor that reports the path is a symbolic link via stat.
    struct SymlinkStatExec;

    #[async_trait]
    impl SshExecutor for SymlinkStatExec {
        async fn exec(
            &self,
            _: &HostConfig,
            program: &str,
            _args: &[&str],
        ) -> anyhow::Result<CommandOutput> {
            match program {
                "python3" => Ok(CommandOutput {
                    stdout: String::new(),
                    stderr: "OSError: Too many levels of symbolic links".into(),
                    exit_code: Some(1),
                }),
                other => anyhow::bail!("unexpected program: {other}"),
            }
        }
    }

    let mut host = HostConfig::local();
    host.name = "remote".into();
    host.host = "remote.example".into();
    host.protocol = crate::synapse::HostProtocol::Ssh;
    host.scout_read_roots = vec!["/tmp".into()];

    let result = super::peek(&host, &SymlinkStatExec, "/tmp/link", false, 3).await;
    assert!(result.is_err(), "remote symlink must be rejected");
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("symbolic link") || msg.contains("symlink") || msg.contains("Too many levels"),
        "error must mention symlink: {msg}"
    );
}

#[tokio::test]
async fn delta_remote_rejects_symlink_path() {
    /// Mock executor that reports the source path is a symbolic link via stat.
    struct SymlinkStatExec;

    #[async_trait]
    impl SshExecutor for SymlinkStatExec {
        async fn exec(
            &self,
            _: &HostConfig,
            program: &str,
            _args: &[&str],
        ) -> anyhow::Result<CommandOutput> {
            match program {
                "python3" => Ok(CommandOutput {
                    stdout: String::new(),
                    stderr: "OSError: Too many levels of symbolic links".into(),
                    exit_code: Some(1),
                }),
                other => anyhow::bail!("unexpected program: {other}"),
            }
        }
    }

    let mut host = HostConfig::local();
    host.name = "remote".into();
    host.host = "remote.example".into();
    host.protocol = crate::synapse::HostProtocol::Ssh;
    host.scout_read_roots = vec!["/tmp".into()];

    let result = super::delta(
        &host,
        &SymlinkStatExec,
        "/tmp/link",
        None,
        None,
        Some("expected content"),
    )
    .await;
    assert!(result.is_err(), "delta must reject remote symlink source");
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("symbolic link") || msg.contains("symlink") || msg.contains("Too many levels"),
        "error must mention symlink: {msg}"
    );
}

// A failed `stat` must fail CLOSED — empty stdout from a nonzero stat must not
// fall through to the file read (the S-M1 guard would otherwise be a no-op on
// hosts without GNU stat or on EPERM).
#[tokio::test]
async fn peek_remote_stat_failure_is_not_silently_bypassed() {
    struct StatFailExec;

    #[async_trait]
    impl SshExecutor for StatFailExec {
        async fn exec(
            &self,
            _: &HostConfig,
            program: &str,
            _args: &[&str],
        ) -> anyhow::Result<CommandOutput> {
            match program {
                "python3" => Ok(CommandOutput {
                    stdout: String::new(),
                    stderr: "Permission denied".into(),
                    exit_code: Some(1),
                }),
                other => panic!("peek must not run unexpected {other}"),
            }
        }
    }

    let mut host = HostConfig::local();
    host.name = "remote".into();
    host.host = "remote.example".into();
    host.protocol = crate::synapse::HostProtocol::Ssh;
    host.scout_read_roots = vec!["/tmp".into()];

    let result = super::peek(&host, &StatFailExec, "/tmp/x", false, 3).await;
    assert!(result.is_err(), "peek must error when stat fails, not read");
}

#[tokio::test]
async fn delta_remote_stat_failure_is_not_silently_bypassed() {
    struct StatFailExec;

    #[async_trait]
    impl SshExecutor for StatFailExec {
        async fn exec(
            &self,
            _: &HostConfig,
            program: &str,
            _args: &[&str],
        ) -> anyhow::Result<CommandOutput> {
            match program {
                "python3" => Ok(CommandOutput {
                    stdout: String::new(),
                    stderr: "Permission denied".into(),
                    exit_code: Some(1),
                }),
                other => panic!("delta must not run unexpected {other}"),
            }
        }
    }

    let mut host = HostConfig::local();
    host.name = "remote".into();
    host.host = "remote.example".into();
    host.protocol = crate::synapse::HostProtocol::Ssh;
    host.scout_read_roots = vec!["/tmp".into()];

    let result = super::delta(&host, &StatFailExec, "/tmp/link", None, None, Some("x")).await;
    assert!(
        result.is_err(),
        "delta must error when stat fails, not read"
    );
}

struct NoopExec;

#[async_trait]
impl SshExecutor for NoopExec {
    async fn exec(&self, _: &HostConfig, _: &str, _: &[&str]) -> anyhow::Result<CommandOutput> {
        Ok(CommandOutput {
            stdout: String::new(),
            stderr: String::new(),
            exit_code: Some(0),
        })
    }
}
