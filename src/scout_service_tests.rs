//! Unit tests for ScoutService — sidecar for src/scout_service.rs.
//!
//! Verifies the help contract and that an injected `HostRepository` is actually
//! used by the scout methods (the propagation invariant the facade must keep).

use super::*;
use crate::synapse::HostConfig;

/// In-memory host repository returning a fixed, distinctive host set.
struct StubHostRepository {
    hosts: Vec<HostConfig>,
}

impl HostRepository for StubHostRepository {
    fn load_hosts(&self) -> anyhow::Result<Vec<HostConfig>> {
        Ok(self.hosts.clone())
    }
}

fn stub_repo() -> Arc<dyn HostRepository> {
    let mut host = HostConfig::local();
    host.name = "stub-node".into();
    Arc::new(StubHostRepository { hosts: vec![host] })
}

#[tokio::test]
async fn test_scout_help_shape() {
    let scout = ScoutService::new(stub_repo());
    let result = scout.help().await.expect("help should succeed");

    assert_eq!(result["tool"], "scout");
    assert_eq!(
        result["actions"],
        serde_json::json!(["nodes", "peek", "exec", "help"])
    );
}

#[tokio::test]
async fn test_scout_nodes_uses_injected_repo() {
    let scout = ScoutService::new(stub_repo());
    let result = scout.nodes().await.expect("nodes should succeed");

    let hosts = result["hosts"]
        .as_array()
        .expect("hosts should be an array");
    assert_eq!(hosts.len(), 1);
    assert_eq!(hosts[0]["name"], "stub-node");
}

#[tokio::test]
async fn test_scout_exec_resolves_through_injected_repo() {
    let scout = ScoutService::new(stub_repo());
    // Unknown host comes from the injected repo (only "stub-node" exists), so
    // resolving "missing" must fail with the repo-driven error.
    let error = scout
        .exec("missing", "/tmp", "echo hi")
        .await
        .expect_err("unknown host should be rejected via injected repo");
    assert!(error.to_string().contains("unknown host"));
}
