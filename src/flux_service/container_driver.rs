//! `FluxService` driver methods for container ops (B8 + B9).
//!
//! This module holds `impl FluxService` blocks that drive host resolution,
//! bollard client acquisition, and multi-host fanout for container operations.
//! Read-only ops delegate to `container_read`; lifecycle ops delegate to
//! `container_lifecycle`. Both pure modules are unit-testable with `MockDockerClient`.

use anyhow::Result;
use futures::StreamExt;
use serde_json::{Map, Value, json};
use std::sync::Arc;

use super::{
    FluxService,
    container_lifecycle::{self, ExecParams, RecreateParams},
    container_read::{self, ListFilters, LogOptions},
    flatten_list_outcome,
};
use crate::docker_client::is_transport_dead;
use crate::elicitation_gate::Confirmer;
use crate::fanout::{FanoutOutcome, fanout};
use crate::scout;

#[cfg(test)]
#[path = "container_driver_tests.rs"]
mod tests;

/// Maximum one-shot stats requests issued per host for an unfiltered call.
pub(crate) const MAX_CONTAINER_STATS_PER_HOST: usize = 200;

struct StatsHostBatch {
    values: Vec<Value>,
    errors: Vec<Value>,
    total: usize,
    requested: usize,
}

fn bounded_stats_ids(containers: &[Value]) -> (Vec<String>, usize) {
    let mut ids = Vec::with_capacity(MAX_CONTAINER_STATS_PER_HOST.min(containers.len()));
    let mut total = 0;
    for id in containers
        .iter()
        .filter_map(|container| container.get("id").and_then(Value::as_str))
    {
        total += 1;
        if ids.len() < MAX_CONTAINER_STATS_PER_HOST {
            ids.push(id.to_owned());
        }
    }
    (ids, total)
}

impl FluxService {
    /// List containers across target host(s), fanning out when `host` is unset.
    /// Returns a flat host-tagged container list with a `partial`/`errors` block.
    pub async fn container_list(&self, host: Option<&str>, filters: ListFilters) -> Result<Value> {
        let hosts = self.target_docker_hosts(host).await?;
        let clients = Arc::clone(&self.docker_clients);
        let outcome = fanout(&hosts, move |h| {
            let clients = Arc::clone(&clients);
            let filters = filters.clone();
            async move {
                let client = clients.client_for(&h).await.map_err(|e| e.to_string())?;
                container_read::list_on_host(client.as_ref(), &h.name, &filters)
                    .await
                    .map_err(|e| {
                        if is_transport_dead(&e) {
                            clients.invalidate(&h);
                        }
                        e.to_string()
                    })
            }
        })
        .await;
        Ok(flatten_list_outcome(outcome, "containers"))
    }

    /// Full-text search containers (name + image + labels) across target host(s).
    pub async fn container_search(&self, host: Option<&str>, query: &str) -> Result<Value> {
        let hosts = self.target_docker_hosts(host).await?;
        let clients = Arc::clone(&self.docker_clients);
        let filters = ListFilters::default();
        let outcome = fanout(&hosts, move |h| {
            let clients = Arc::clone(&clients);
            let filters = filters.clone();
            async move {
                let client = clients.client_for(&h).await.map_err(|e| e.to_string())?;
                container_read::list_on_host(client.as_ref(), &h.name, &filters)
                    .await
                    .map_err(|e| {
                        if is_transport_dead(&e) {
                            clients.invalidate(&h);
                        }
                        e.to_string()
                    })
            }
        })
        .await;
        let mut result = flatten_list_outcome(outcome, "containers");
        if let Some(arr) = result.get("containers").and_then(Value::as_array) {
            let matches: Vec<Value> = arr
                .iter()
                .filter(|c| container_read::search_matches(c, query))
                .cloned()
                .collect();
            // `flatten_list_outcome` always returns a `Value::Object`, so this
            // branch is always taken. Prefer `if let` over `expect` (H-4).
            if let Some(obj) = result.as_object_mut() {
                obj.insert("count".into(), json!(matches.len()));
                obj.insert("containers".into(), json!(matches));
                obj.insert("query".into(), json!(query));
            }
        }
        Ok(result)
    }

    /// One-shot stats for one container, or every container on the host(s) when
    /// `container_id` is `None`.
    pub async fn container_stats(
        &self,
        host: Option<&str>,
        container_id: Option<&str>,
    ) -> Result<Value> {
        if let Some(id) = container_id {
            // Single container: find-host then one-shot stats.
            return self
                .find_host_op(host, id, |client, host_name, id| {
                    Box::pin(container_read::stats_on_host(client, host_name, id))
                })
                .await;
        }
        // No id: fan out, collect per-host all-container stats.
        let hosts = self.target_docker_hosts(host).await?;
        let clients = Arc::clone(&self.docker_clients);
        let outcome = fanout(&hosts, move |h| {
            let clients = Arc::clone(&clients);
            async move {
                let client = clients.client_for(&h).await.map_err(|e| e.to_string())?;
                let containers =
                    container_read::list_on_host(client.as_ref(), &h.name, &ListFilters::default())
                        .await
                        .map_err(|e| {
                            if is_transport_dead(&e) {
                                clients.invalidate(&h);
                            }
                            e.to_string()
                        })?;
                let (ids, total) = bounded_stats_ids(&containers);
                let requested = ids.len();
                let mut stats: Vec<(usize, Result<Value, Value>)> =
                    futures::stream::iter(ids.into_iter().enumerate())
                        .map(|(index, id)| {
                            let client = Arc::clone(&client);
                            let host_name = h.name.clone();
                            async move {
                                let value = match container_read::stats_on_host(
                                    client.as_ref(),
                                    &host_name,
                                    &id,
                                )
                                .await
                                {
                                    Ok(value) => Ok(value),
                                    Err(error) => Err(json!({
                                        "host": host_name,
                                        "container_id": id,
                                        "error": error.to_string(),
                                    })),
                                };
                                (index, value)
                            }
                        })
                        .buffer_unordered(8)
                        .collect()
                        .await;
                stats.sort_unstable_by_key(|(index, _)| *index);
                let mut values = Vec::new();
                let mut errors = Vec::new();
                for (_, result) in stats {
                    match result {
                        Ok(value) => values.push(value),
                        Err(error) => errors.push(error),
                    }
                }
                Ok::<_, String>(StatsHostBatch {
                    values,
                    errors,
                    total,
                    requested,
                })
            }
        })
        .await;
        Ok(flatten_stats_outcome(outcome))
    }

    /// Inspect a single container (full or `summary`), resolving its host.
    pub async fn container_inspect(
        &self,
        host: Option<&str>,
        container_id: &str,
        summary: bool,
    ) -> Result<Value> {
        self.find_host_op(host, container_id, move |client, host_name, id| {
            Box::pin(container_read::inspect_on_host(
                client, host_name, id, summary,
            ))
        })
        .await
    }

    /// Show running processes (`top`) in a single container, resolving its host.
    pub async fn container_top(&self, host: Option<&str>, container_id: &str) -> Result<Value> {
        self.find_host_op(host, container_id, |client, host_name, id| {
            Box::pin(container_read::top_on_host(client, host_name, id))
        })
        .await
    }

    /// Fetch one-shot logs for a single container, resolving its host.
    pub async fn container_logs(
        &self,
        host: Option<&str>,
        container_id: &str,
        opts: LogOptions,
    ) -> Result<Value> {
        let bollard_opts = container_read::build_logs_options(&opts)?;
        let grep = opts.grep.clone();
        let id = container_id.to_owned();
        self.find_host_op(host, container_id, move |client, host_name, _| {
            let bollard_opts = bollard_opts.clone();
            let grep = grep.clone();
            let id = id.clone();
            let host_name = host_name.to_owned();
            Box::pin(async move {
                let lines = container_read::collect_log_lines(client, &id, bollard_opts).await?;
                let lines = container_read::grep_lines(lines, grep.as_deref());
                Ok(container_read::logs_value(&host_name, &id, lines))
            })
        })
        .await
    }

    /// Run a single-container read against the named host, or fan out to locate
    /// a unique owning host. Ambiguous matches fail closed.
    pub(super) async fn find_host_op<F>(
        &self,
        host: Option<&str>,
        container_id: &str,
        op: F,
    ) -> Result<Value>
    where
        F: for<'a> Fn(
                &'a dyn crate::docker_client::ContainerOps,
                &'a str,
                &'a str,
            ) -> std::pin::Pin<
                Box<
                    dyn std::future::Future<Output = Result<Value, bollard::errors::Error>>
                        + Send
                        + 'a,
                >,
            > + Sync,
    {
        let hosts = self.target_hosts(host)?;
        // Named host → target directly (surface its error verbatim).
        if host.is_some() {
            let h = &hosts[0];
            let client = self.docker_clients.client_for(h).await?;
            return op(client.as_ref(), &h.name, container_id)
                .await
                .map_err(|e| {
                    // Evict stale SSH-forwarded socket so next call rebuilds (T-H3).
                    if is_transport_dead(&e) {
                        self.docker_clients.invalidate(h);
                    }
                    anyhow::Error::from(e)
                });
        }
        // Unspecified read → probe hosts concurrently, then resolve in stable
        // topology order. Multiple matches are ambiguous and fail closed.
        let mut probes = futures::stream::iter(hosts.into_iter().enumerate())
            .map(|(index, h)| {
                let op = &op;
                async move {
                    let result = match self.docker_clients.client_for(&h).await {
                        Ok(client) => {
                            op(client.as_ref(), &h.name, container_id)
                                .await
                                .map_err(|error| {
                                    if is_transport_dead(&error) {
                                        self.docker_clients.invalidate(&h);
                                    }
                                    anyhow::Error::from(error)
                                })
                        }
                        Err(error) => Err(error),
                    };
                    (index, h.name, result)
                }
            })
            .buffer_unordered(8);
        let mut errors = Vec::new();
        let mut matches = Vec::new();
        while let Some((index, host_name, result)) = probes.next().await {
            match result {
                Ok(value) => matches.push((index, host_name, value)),
                Err(error) => errors.push(format!("{host_name}: {error}")),
            }
        }
        if let Some(value) = resolve_unique_host_match(container_id, matches)? {
            return Ok(value);
        }
        Err(anyhow::anyhow!(
            "container {container_id} not found on any host ({})",
            errors.join("; ")
        ))
    }

    // ── B9: container lifecycle ────────────────────────────────────────────

    /// Perform a simple lifecycle action (start/stop/restart/pause/resume),
    /// resolving the owning host when `host` is unspecified.
    ///
    /// `stop` is destructive — the caller MUST pass a gated `confirmer`.
    pub async fn container_lifecycle(
        &self,
        host: Option<&str>,
        container_id: &str,
        subaction: &str,
        confirmer: &dyn Confirmer,
    ) -> Result<Value> {
        let host = host.ok_or_else(|| {
            anyhow::anyhow!("host is required for container {subaction} operations")
        })?;
        // Gate before any IO.
        if subaction == "stop" {
            confirmer
                .require("container stop", &format!("stop container {container_id}"))
                .await
                .map_err(|e| anyhow::anyhow!(e))?;
        }
        let subaction = subaction.to_owned();
        self.find_host_op(Some(host), container_id, move |client, host_name, id| {
            let sub = subaction.clone();
            Box::pin(async move {
                container_lifecycle::lifecycle_action_on_host(client, host_name, id, &sub).await
            })
        })
        .await
    }

    /// Pull the latest image for the given container's image on a single host.
    /// Resolves the owning host first to discover the image ref.
    /// Non-gated (parity with synapse-mcp).
    pub async fn container_pull(&self, host: Option<&str>, container_id: &str) -> Result<Value> {
        let host = host.ok_or_else(|| anyhow::anyhow!("host is required for container pull"))?;
        // Step 1: Inspect to get the image ref (find-host pattern).
        let inspect_val = self
            .find_host_op(Some(host), container_id, |client, host_name, id| {
                Box::pin(container_read::inspect_on_host(
                    client, host_name, id, false,
                ))
            })
            .await?;

        let host_name = inspect_val
            .get("host")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("inspect returned no host"))?
            .to_owned();
        let image_ref = inspect_val
            .pointer("/container/Config/Image")
            .or_else(|| inspect_val.pointer("/container/config/Image"))
            .or_else(|| inspect_val.pointer("/container/config/image"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();

        // Step 2: Pull on the resolved host.
        let h = scout::resolve_host(self.host_repo.as_ref(), &host_name)?;
        let client = self.docker_clients.client_for(&h).await?;
        container_lifecycle::pull_image_on_host(client.as_ref(), &h.name, &image_ref)
            .await
            .map_err(Into::into)
    }

    /// Recreate a container (inspect → pull → stop → remove → create → start).
    /// DESTRUCTIVE — gated via the B5 Confirmer before any IO.
    pub async fn container_recreate(
        &self,
        host: Option<&str>,
        container_id: &str,
        params: RecreateParams,
        confirmer: &dyn Confirmer,
    ) -> Result<Value> {
        let host =
            host.ok_or_else(|| anyhow::anyhow!("host is required for container recreate"))?;
        let h = self.target_hosts(Some(host))?[0].clone();

        // Gate before any IO.
        confirmer
            .require(
                "container recreate",
                &format!("recreate container {container_id} on {}", h.name),
            )
            .await
            .map_err(|e| anyhow::anyhow!(e))?;

        let client = self.docker_clients.client_for(&h).await?;
        container_lifecycle::recreate_on_host(client.as_ref(), &h.name, container_id, &params)
            .await
            .map_err(Into::into)
    }

    /// Execute a command inside a container (one-shot exec, 3-step bollard).
    /// DESTRUCTIVE — gated via the B5 Confirmer before any IO.
    pub async fn container_exec(
        &self,
        host: Option<&str>,
        params: ExecParams,
        confirmer: &dyn Confirmer,
    ) -> Result<Value> {
        let host = host.ok_or_else(|| anyhow::anyhow!("host is required for container exec"))?;
        let container_id = params.container_id.clone();
        // Gate before any IO.
        confirmer
            .require(
                "container exec",
                &format!(
                    "{} on {}",
                    params.command.first().map(|s| s.as_str()).unwrap_or(""),
                    container_id
                ),
            )
            .await
            .map_err(|e| anyhow::anyhow!(e))?;

        self.find_host_op(Some(host), &container_id, move |client, host_name, _| {
            let params = params.clone();
            Box::pin(
                async move { container_lifecycle::exec_on_host(client, host_name, &params).await },
            )
        })
        .await
    }
}

fn flatten_stats_outcome(outcome: FanoutOutcome<StatsHostBatch, String>) -> Value {
    let mut stats = Vec::new();
    let mut container_errors = Vec::new();
    let mut truncated_hosts = Vec::new();
    for (host_name, batch) in outcome.ok_results() {
        stats.extend(batch.values.iter().cloned());
        container_errors.extend(batch.errors.iter().cloned());
        if batch.total > batch.requested {
            truncated_hosts.push(json!({
                "host": host_name,
                "total_containers": batch.total,
                "requested": batch.requested,
                "successful": batch.values.len(),
            }));
        }
    }
    let errors: Map<String, Value> = outcome
        .err_results()
        .iter()
        .map(|(host_name, error)| (host_name.clone(), json!(error)))
        .collect();
    let mut response = json!({
        "count": stats.len(),
        "stats": stats,
        "partial": outcome.is_partial() || !container_errors.is_empty(),
    });
    if let Some(object) = response.as_object_mut() {
        if !errors.is_empty() {
            object.insert("errors".into(), Value::Object(errors));
        }
        if !truncated_hosts.is_empty() {
            object.insert("truncated".into(), Value::Bool(true));
            object.insert(
                "max_containers_per_host".into(),
                json!(MAX_CONTAINER_STATS_PER_HOST),
            );
            object.insert("truncated_hosts".into(), Value::Array(truncated_hosts));
        }
        if !container_errors.is_empty() {
            object.insert("container_errors".into(), Value::Array(container_errors));
        }
    }
    response
}

fn resolve_unique_host_match(
    container_id: &str,
    mut matches: Vec<(usize, String, Value)>,
) -> Result<Option<Value>> {
    matches.sort_unstable_by_key(|(index, _, _)| *index);
    if matches.len() > 1 {
        let hosts = matches
            .iter()
            .map(|(_, host, _)| host.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        anyhow::bail!("container {container_id} is ambiguous across hosts ({hosts}); specify host");
    }
    Ok(matches.pop().map(|(_, _, value)| value))
}
