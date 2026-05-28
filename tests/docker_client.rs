//! Integration tests for the bollard Docker client.
//!
//! Local-daemon tests skip cleanly (return early, print a note) when
//! `/var/run/docker.sock` is absent, so CI without docker stays green.
//!
//! The **remote** SSH-forwarded path (`BollardClient::connect_remote`) is
//! compile-checked everywhere but only gets live coverage when a reachable
//! remote docker host is provided via the `SYNAPSE2_TEST_REMOTE_HOST` env var
//! (CI typically lacks one, so that test skips). Set it to an SSH host name from
//! `~/.ssh/config` that runs docker to exercise the forward end-to-end.

use std::path::Path;
use std::sync::Arc;

use synapse2::docker_client::{
    ContainerOps, DockerClientCache, ImageOps, NetworkOps, SystemOps, VolumeOps,
};
use synapse2::synapse::HostConfig;

/// Returns false (and prints a skip note) when there is no local docker socket.
fn docker_available() -> bool {
    if Path::new("/var/run/docker.sock").exists() {
        return true;
    }
    eprintln!("skipping: /var/run/docker.sock not present");
    false
}

#[tokio::test]
async fn local_info_returns_typed_struct() {
    if !docker_available() {
        return;
    }
    let cache = DockerClientCache::new();
    let client = cache
        .client_for(&HostConfig::local())
        .await
        .expect("connect to local docker");

    // `info()` returns a typed `bollard::secret::SystemInfo`, not a JSON blob.
    let info: bollard::models::SystemInfo = client.info().await.expect("docker info");
    // A real daemon reports a non-empty id or at least a containers count.
    assert!(
        info.id.is_some() || info.containers.is_some(),
        "expected SystemInfo to carry daemon fields"
    );
}

#[tokio::test]
async fn local_list_calls_succeed() {
    if !docker_available() {
        return;
    }
    let cache = DockerClientCache::new();
    let client = cache
        .client_for(&HostConfig::local())
        .await
        .expect("connect to local docker");

    // These must not error against a healthy daemon (empty results are fine).
    client.list_containers(None).await.expect("list containers");
    client.list_images(None).await.expect("list images");
    client.list_networks(None).await.expect("list networks");
    client.list_volumes(None).await.expect("list volumes");
    client.ping().await.expect("ping");
}

#[tokio::test]
async fn cache_returns_same_instance_for_repeated_lookup() {
    if !docker_available() {
        return;
    }
    let cache = DockerClientCache::new();
    let host = HostConfig::local();

    let a = cache.client_for(&host).await.expect("first lookup");
    let b = cache.client_for(&host).await.expect("second lookup");

    // Same cached client (same allocation) for repeated lookups.
    assert!(Arc::ptr_eq(&a, &b));
    assert_eq!(cache.len(), 1);
}

#[tokio::test]
async fn invalidate_frees_then_rebuilds() {
    if !docker_available() {
        return;
    }
    let cache = DockerClientCache::new();
    let host = HostConfig::local();

    let first = cache.client_for(&host).await.expect("first build");
    cache.invalidate(&host);
    assert!(cache.is_empty(), "entry should be evicted");

    let second = cache.client_for(&host).await.expect("rebuild");
    // A fresh client allocation after eviction.
    assert!(!Arc::ptr_eq(&first, &second));
}

/// Remote (SSH-forwarded) path against a real host. Skips unless
/// `SYNAPSE2_TEST_REMOTE_HOST` names a reachable SSH host (from `~/.ssh/config`)
/// that runs docker. Exercises the bead's "B1-forwarded remote socket — same
/// calls, same shape" requirement when infra is available.
#[tokio::test]
async fn remote_forwarded_socket_same_shape() {
    let Ok(host_name) = std::env::var("SYNAPSE2_TEST_REMOTE_HOST") else {
        eprintln!("skipping: set SYNAPSE2_TEST_REMOTE_HOST to a docker-running SSH host");
        return;
    };

    // Minimal SSH host config; relies on ~/.ssh/config for user/key/port.
    let host = HostConfig {
        name: host_name.clone(),
        host: host_name,
        protocol: synapse2::synapse::HostProtocol::Ssh,
        ..HostConfig::local()
    };

    let cache = DockerClientCache::new();
    let client = cache
        .client_for(&host)
        .await
        .expect("connect to remote docker via forwarded socket");

    // Same calls as the local suite, same typed shapes.
    let _info: bollard::models::SystemInfo = client.info().await.expect("remote docker info");
    client
        .list_containers(None)
        .await
        .expect("remote list containers");
    client.list_images(None).await.expect("remote list images");
    client
        .list_networks(None)
        .await
        .expect("remote list networks");
    client
        .list_volumes(None)
        .await
        .expect("remote list volumes");
    client.ping().await.expect("remote ping");

    // Same cached instance on repeat lookup (dedup through OnceCell).
    let again = cache.client_for(&host).await.expect("second lookup");
    assert!(Arc::ptr_eq(&client, &again));

    // Explicit teardown of the forward (best-effort; cache still holds a clone).
    cache.invalidate(&host);
}
