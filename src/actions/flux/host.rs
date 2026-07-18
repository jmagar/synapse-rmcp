//! Dispatch for `flux host` actions.

use anyhow::Result;
use serde_json::Value;

use crate::actions::{ValidationError, require_field};
use crate::app::SynapseService;

use super::HostArgs;

pub(crate) async fn dispatch_flux_host(service: &SynapseService, args: &HostArgs) -> Result<Value> {
    let flux = service.flux();
    let host = args.host.as_deref();
    match args.subaction.as_str() {
        "status" => flux.host_status(host).await,
        "info" => flux.host_info(host).await,
        "uptime" => flux.host_uptime(host).await,
        "resources" => flux.host_resources(host).await,
        "services" => {
            let h = require_field(&args.host, "host")?;
            flux.host_services(h, args.state.as_deref(), args.service.as_deref())
                .await
        }
        "network" => flux.host_network(host).await,
        "mounts" => {
            let h = require_field(&args.host, "host")?;
            flux.host_mounts(h).await
        }
        "ports" => {
            let h = require_field(&args.host, "host")?;
            let limit = args.limit.map(|v| v as usize);
            let offset = args.offset.map(|v| v as usize);
            flux.host_ports(h, args.protocol.as_deref(), limit, offset)
                .await
        }
        "doctor" => {
            let h = require_field(&args.host, "host")?;
            let checks: Vec<String> = match &args.checks {
                Some(s) if !s.is_empty() => s.split(',').map(|c| c.trim().to_owned()).collect(),
                _ => crate::flux_service::host::DEFAULT_DOCTOR_CHECKS
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
            };
            flux.host_doctor(h, checks).await
        }
        other => Err(ValidationError::UnknownAction {
            action: format!("host:{other}"),
        }
        .into()),
    }
}
