//! Dispatch for `flux container` actions.

use anyhow::Result;
use serde_json::Value;

use crate::actions::{ValidationError, require_container_id, require_field};
use crate::app::SynapseService;

use super::ContainerArgs;

pub(crate) async fn dispatch_flux_container(
    service: &SynapseService,
    args: &ContainerArgs,
    confirmer: &dyn crate::elicitation_gate::Confirmer,
) -> Result<Value> {
    use crate::flux_service::container_lifecycle::{
        EXEC_TIMEOUT_DEFAULT_MS, ExecParams, RecreateParams,
    };
    use crate::flux_service::container_read::{DEFAULT_LOG_LINES, ListFilters, LogOptions};
    let flux = service.flux();
    let host = args.host.as_deref();
    match args.subaction.as_str() {
        "list" => {
            let filters = ListFilters {
                state: args.state.clone(),
                name_filter: args.name_filter.clone(),
                image_filter: args.image_filter.clone(),
                label_filter: args.label_filter.clone(),
            };
            flux.container_list(host, filters).await
        }
        "search" => {
            let q = args.query.as_deref().ok_or(ValidationError::MissingField {
                field: "query".into(),
            })?;
            flux.container_search(host, q).await
        }
        "stats" => {
            flux.container_stats(host, args.container_id.as_deref())
                .await
        }
        "inspect" => {
            flux.container_inspect(
                host,
                require_container_id(&args.container_id)?,
                args.summary.unwrap_or(false),
            )
            .await
        }
        "top" => {
            flux.container_top(host, require_container_id(&args.container_id)?)
                .await
        }
        "logs" => {
            let opts = LogOptions {
                lines: args.lines.unwrap_or(DEFAULT_LOG_LINES),
                since: args.since.clone(),
                until: args.until.clone(),
                grep: args.grep.clone(),
                stream: args.stream.clone().unwrap_or_else(|| "both".to_owned()),
            };
            flux.container_logs(host, require_container_id(&args.container_id)?, opts)
                .await
        }
        sa @ ("start" | "stop" | "restart" | "pause" | "resume") => {
            flux.container_lifecycle(
                Some(require_field(&args.host, "host")?),
                require_container_id(&args.container_id)?,
                sa,
                confirmer,
            )
            .await
        }
        "pull" => {
            flux.container_pull(
                Some(require_field(&args.host, "host")?),
                require_container_id(&args.container_id)?,
            )
            .await
        }
        "recreate" => {
            let params = RecreateParams {
                pull: args.pull.unwrap_or(true),
            };
            flux.container_recreate(
                Some(require_field(&args.host, "host")?),
                require_container_id(&args.container_id)?,
                params,
                confirmer,
            )
            .await
        }
        "exec" => {
            if args.command.is_empty() {
                return Err(ValidationError::MissingField {
                    field: "command".into(),
                }
                .into());
            }
            let params = ExecParams {
                container_id: require_container_id(&args.container_id)?.to_owned(),
                command: args.command.clone(),
                user: args.exec_user.clone(),
                workdir: args.exec_workdir.clone(),
                timeout_ms: args.exec_timeout_ms.unwrap_or(EXEC_TIMEOUT_DEFAULT_MS),
            };
            flux.container_exec(Some(require_field(&args.host, "host")?), params, confirmer)
                .await
        }
        other => Err(ValidationError::UnknownAction {
            action: format!("container:{other}"),
        }
        .into()),
    }
}
