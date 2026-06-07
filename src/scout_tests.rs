use anyhow::Result;

use crate::host_config::HostRepository;
use crate::synapse::HostConfig;

struct StaticRepo(Vec<HostConfig>);

impl HostRepository for StaticRepo {
    fn load_hosts(&self) -> Result<Vec<HostConfig>> {
        Ok(self.0.clone())
    }
}

#[test]
fn nodes_returns_configured_hosts() {
    let value = super::nodes(&StaticRepo(vec![HostConfig::local()]))
        .expect("nodes should serialize configured hosts");

    assert_eq!(value["hosts"][0]["name"], "local");
}

#[test]
fn resolve_host_reports_unknown_host() {
    let err = super::resolve_host(&StaticRepo(Vec::new()), "missing").unwrap_err();

    assert_eq!(err.to_string(), "unknown host: missing");
}
