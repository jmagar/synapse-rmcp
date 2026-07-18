//! Dispatch for `flux compose` actions.

use anyhow::Result;
use serde_json::{Value, json};

use crate::actions::{ValidationError, require_field};
use crate::app::SynapseService;

use super::ComposeArgs;

pub(crate) async fn dispatch_flux_compose(
    service: &SynapseService,
    args: &ComposeArgs,
    confirmer: &dyn crate::elicitation_gate::Confirmer,
) -> Result<Value> {
    use crate::flux_service::compose_ops::{ComposeLogOptions, DownArgs};
    let flux = service.flux();
    let host = require_field(&args.host, "host")?;
    match args.subaction.as_str() {
        "list" => {
            let projects = flux.compose_list(host).await?;
            let items: Vec<Value> = projects
                .iter()
                .map(|p| serde_json::to_value(p).unwrap_or(Value::Null))
                .collect();
            Ok(json!({
                "host": host,
                "count": items.len(),
                "projects": items,
            }))
        }
        "refresh" => {
            flux.compose_refresh(Some(host));
            Ok(json!({ "host": host, "refreshed": true }))
        }
        "status" => {
            let project = require_field(&args.project, "project")?;
            flux.compose_status(host, project, args.service.as_deref())
                .await
        }
        "up" => {
            let project = require_field(&args.project, "project")?;
            flux.compose_up(host, project).await
        }
        "down" => {
            let project = require_field(&args.project, "project")?;
            let down_args = DownArgs {
                remove_volumes: args.remove_volumes.unwrap_or(false),
                force: args.force.unwrap_or(false),
            };
            flux.compose_down(host, project, down_args, confirmer).await
        }
        "restart" => {
            let project = require_field(&args.project, "project")?;
            flux.compose_restart(host, project, confirmer).await
        }
        "recreate" => {
            let project = require_field(&args.project, "project")?;
            flux.compose_recreate(host, project, confirmer).await
        }
        "logs" => {
            let project = require_field(&args.project, "project")?;
            let opts = ComposeLogOptions {
                lines: args.lines,
                since: args.since.clone(),
                service: args.service.clone(),
            };
            flux.compose_logs(host, project, opts).await
        }
        "build" => {
            let project = require_field(&args.project, "project")?;
            flux.compose_build(host, project, args.service.as_deref())
                .await
        }
        "pull" => {
            let project = require_field(&args.project, "project")?;
            flux.compose_pull(host, project, args.service.as_deref())
                .await
        }
        other => Err(ValidationError::UnknownAction {
            action: format!("compose:{other}"),
        }
        .into()),
    }
}
