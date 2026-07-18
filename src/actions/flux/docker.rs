//! Dispatch for `flux docker` actions.

use anyhow::Result;
use serde_json::Value;

use crate::actions::{ValidationError, require_field};
use crate::app::SynapseService;

use super::DockerArgs;

pub(crate) async fn dispatch_flux_docker(
    service: &SynapseService,
    args: &DockerArgs,
    confirmer: &dyn crate::elicitation_gate::Confirmer,
) -> Result<Value> {
    use crate::flux_service::docker::{PruneTarget, build_args};
    let flux = service.flux();
    let host = args.host.as_deref();
    match args.subaction.as_str() {
        "info" => flux.docker_info(host).await,
        "df" => flux.docker_df(host).await,
        "images" => {
            flux.docker_images(host, args.dangling_only.unwrap_or(false))
                .await
        }
        "networks" => flux.docker_networks(host).await,
        "volumes" => flux.docker_volumes(host).await,
        "pull" => {
            let image = require_field(&args.image, "image")?;
            flux.docker_pull(require_field(&args.host, "host")?, image)
                .await
        }
        "build" => {
            let context = require_field(&args.context, "context")?;
            let tag = require_field(&args.tag, "tag")?;
            let built = build_args(
                context,
                tag,
                args.dockerfile.as_deref(),
                args.no_cache.unwrap_or(false),
            )?;
            flux.docker_build(require_field(&args.host, "host")?, built, confirmer)
                .await
        }
        "rmi" => {
            let image = require_field(&args.image, "image")?;
            let force = args.force.unwrap_or(false);
            if !force {
                return Err(ValidationError::MissingField {
                    field: "force (rmi requires force=true)".into(),
                }
                .into());
            }
            flux.docker_rmi(require_field(&args.host, "host")?, image, force, confirmer)
                .await
        }
        "prune" => {
            let target_str = require_field(&args.prune_target, "prune_target")?;
            let target = PruneTarget::parse(target_str)?;
            if !args.force.unwrap_or(false) {
                return Err(ValidationError::MissingField {
                    field: "force (prune requires force=true)".into(),
                }
                .into());
            }
            flux.docker_prune(require_field(&args.host, "host")?, target, confirmer)
                .await
        }
        other => Err(ValidationError::UnknownAction {
            action: format!("docker:{other}"),
        }
        .into()),
    }
}
