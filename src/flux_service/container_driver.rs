//! `FluxService` driver methods for container read-only ops (B8).
//!
//! This module holds `impl FluxService` blocks that drive host resolution,
//! bollard client acquisition, and multi-host fanout for container operations.
//! The per-host logic lives in the pure `container_read` sibling module so it
//! stays unit-testable with `MockDockerClient`.

use anyhow::Result;
use serde_json::{json, Value};

use super::{
    container_read::{self, ListFilters, LogOptions},
    flatten_list_outcome, FluxService,
};
use crate::fanout::fanout;

impl FluxService {
    /// List containers across target host(s), fanning out when `host` is unset.
    /// Returns a flat host-tagged container list with a `partial`/`errors` block.
    pub async fn container_list(&self, host: Option<&str>, filters: ListFilters) -> Result<Value> {
        let hosts = self.target_hosts(host)?;
        let clients = &self.docker_clients;
        let outcome = fanout(&hosts, |h| {
            let filters = filters.clone();
            async move {
                let client = clients.client_for(&h).await.map_err(|e| e.to_string())?;
                container_read::list_on_host(client.as_ref(), &h.name, &filters)
                    .await
                    .map_err(|e| e.to_string())
            }
        })
        .await;
        Ok(flatten_list_outcome(outcome, "containers"))
    }

    /// Full-text search containers (name + image + labels) across target host(s).
    pub async fn container_search(&self, host: Option<&str>, query: &str) -> Result<Value> {
        let hosts = self.target_hosts(host)?;
        let clients = &self.docker_clients;
        let filters = ListFilters::default();
        let outcome = fanout(&hosts, |h| {
            let filters = filters.clone();
            async move {
                let client = clients.client_for(&h).await.map_err(|e| e.to_string())?;
                container_read::list_on_host(client.as_ref(), &h.name, &filters)
                    .await
                    .map_err(|e| e.to_string())
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
            let obj = result.as_object_mut().expect("flatten produces an object");
            obj.insert("count".into(), json!(matches.len()));
            obj.insert("containers".into(), json!(matches));
            obj.insert("query".into(), json!(query));
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
        let hosts = self.target_hosts(host)?;
        let clients = &self.docker_clients;
        let outcome = fanout(&hosts, |h| async move {
            let client = clients.client_for(&h).await.map_err(|e| e.to_string())?;
            let containers =
                container_read::list_on_host(client.as_ref(), &h.name, &ListFilters::default())
                    .await
                    .map_err(|e| e.to_string())?;
            let mut stats = Vec::new();
            for c in &containers {
                if let Some(id) = c.get("id").and_then(Value::as_str) {
                    if let Ok(s) = container_read::stats_on_host(client.as_ref(), &h.name, id).await
                    {
                        stats.push(s);
                    }
                }
            }
            Ok::<_, String>(stats)
        })
        .await;
        Ok(flatten_list_outcome(outcome, "stats"))
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

    /// Run a single-container op against the named host, or fan out to find the
    /// owning host (first match wins) when `host` is unspecified.
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
                dyn std::future::Future<Output = Result<Value, bollard::errors::Error>> + Send + 'a,
            >,
        >,
    {
        let hosts = self.target_hosts(host)?;
        // Named host → target directly (surface its error verbatim).
        if host.is_some() {
            let h = &hosts[0];
            let client = self.docker_clients.client_for(h).await?;
            return op(client.as_ref(), &h.name, container_id)
                .await
                .map_err(Into::into);
        }
        // Unspecified → probe hosts, first that has the container wins.
        let mut errors: Vec<String> = Vec::new();
        for h in &hosts {
            match self.docker_clients.client_for(h).await {
                Ok(client) => match op(client.as_ref(), &h.name, container_id).await {
                    Ok(value) => return Ok(value),
                    Err(e) => errors.push(format!("{}: {e}", h.name)),
                },
                Err(e) => errors.push(format!("{}: {e}", h.name)),
            }
        }
        Err(anyhow::anyhow!(
            "container {container_id} not found on any host ({})",
            errors.join("; ")
        ))
    }
}
