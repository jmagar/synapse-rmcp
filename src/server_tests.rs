use super::*;
use crate::config::AuthConfig;

fn config(host: &str) -> Config {
    Config {
        mcp: McpConfig {
            host: host.into(),
            ..McpConfig::default()
        },
    }
}

#[test]
fn loopback_bind_is_loopback_dev_without_credentials() {
    let config = config("127.0.0.1");
    assert_eq!(
        resolve_auth_policy_kind(&config).unwrap(),
        AuthPolicyKind::LoopbackDev
    );
}

#[test]
fn non_loopback_no_auth_is_rejected() {
    let mut config = config("0.0.0.0");
    config.mcp.no_auth = true;
    let error = resolve_auth_policy_kind(&config).unwrap_err();
    assert!(error.to_string().contains("SYNAPSE_MCP_NO_AUTH=true"));
}

#[test]
fn non_loopback_no_auth_with_gateway_is_trusted_gateway_unscoped() {
    let mut config = config("0.0.0.0");
    config.mcp.no_auth = true;
    config.mcp.trusted_gateway = true;
    assert_eq!(
        resolve_auth_policy_kind(&config).unwrap(),
        AuthPolicyKind::TrustedGatewayUnscoped
    );
}

#[test]
fn non_loopback_gateway_without_local_credentials_is_supported() {
    let mut config = config("0.0.0.0");
    config.mcp.trusted_gateway = true;
    assert_eq!(
        resolve_auth_policy_kind(&config).unwrap(),
        AuthPolicyKind::TrustedGatewayUnscoped
    );
}

#[test]
fn non_loopback_bearer_token_mounts_bearer_policy() {
    let mut config = config("0.0.0.0");
    config.mcp.api_token = Some("secret".into());
    assert_eq!(
        resolve_auth_policy_kind(&config).unwrap(),
        AuthPolicyKind::MountedBearer
    );
}

#[test]
fn static_bearer_scopes_are_read_only() {
    let scopes = static_bearer_scopes();
    assert_eq!(scopes, vec![crate::actions::READ_SCOPE.to_owned()]);
    assert!(
        !scopes
            .iter()
            .any(|scope| scope == crate::actions::WRITE_SCOPE)
    );
}

#[test]
fn non_loopback_oauth_mounts_oauth_policy() {
    let mut config = config("0.0.0.0");
    config.mcp.auth = AuthConfig {
        mode: AuthMode::OAuth,
        ..AuthConfig::default()
    };
    assert_eq!(
        resolve_auth_policy_kind(&config).unwrap(),
        AuthPolicyKind::MountedOAuth
    );
}

#[test]
fn non_loopback_without_auth_is_rejected() {
    let config = config("0.0.0.0");
    let error = resolve_auth_policy_kind(&config).unwrap_err();
    assert!(error.to_string().contains("without authentication"));
}

#[test]
fn invalid_public_url_is_rejected() {
    let mut config = config("0.0.0.0");
    config.mcp.auth.public_url = Some("not a url".into());
    let error = resolve_auth_policy_kind(&config).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("SYNAPSE_MCP_PUBLIC_URL is invalid")
    );
}

#[test]
fn wildcard_public_url_is_rejected() {
    let mut config = config("0.0.0.0");
    config.mcp.auth.public_url = Some("https://*.synapse2.com".into());
    let error = resolve_auth_policy_kind(&config).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("SYNAPSE_MCP_PUBLIC_URL must not contain wildcard hosts")
    );
}
