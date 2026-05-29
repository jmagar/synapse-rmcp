//! Unit tests for SynapseService — sidecar file for src/app.rs
//!
//! Declared in app.rs as:
//! ```rust
//! #[cfg(test)]
//! #[path = "app_tests.rs"]
//! mod tests;
//! ```
//!
//! The facade tests verify that `SynapseService` correctly delegates to its
//! transport client and sub-services. Domain-specific behavior lives in the
//! sub-service sidecars (flux_service_tests.rs, scout_service_tests.rs) and the
//! scaffold contract tests live in scaffold_tests.rs.

use super::*;
use crate::{config::SynapseConfig, synapse2::SynapseClient};

/// Build a stub SynapseService for testing without real credentials.
fn stub_service() -> SynapseService {
    let client = SynapseClient::new(&SynapseConfig {
        api_url: "http://localhost:1/stub".to_string(),
        api_key: "test-key".to_string(),
    })
    .expect("stub client should always build");
    SynapseService::new(client)
}

#[tokio::test]
async fn test_service_greet_delegates_to_client() {
    let service = stub_service();
    let result = service.greet(None).await.expect("greet should succeed");

    assert!(
        result.get("greeting").is_some(),
        "service greet should return greeting field"
    );
}

#[tokio::test]
async fn test_service_greet_with_name_passes_name_through() {
    let service = stub_service();
    let result = service
        .greet(Some("Bob"))
        .await
        .expect("greet Bob should succeed");

    let greeting = result
        .get("greeting")
        .and_then(|v| v.as_str())
        .expect("greeting field should be present");

    assert!(
        greeting.contains("Bob"),
        "service should pass name through to client; got: {greeting}"
    );
}

#[tokio::test]
async fn test_service_echo_returns_exact_message() {
    let service = stub_service();
    let msg = "service layer echo test";
    let result = service.echo(msg).await.expect("echo should succeed");

    let echo = result
        .get("echo")
        .and_then(|v| v.as_str())
        .expect("echo field should be present");

    assert_eq!(
        echo, msg,
        "service echo should return the input message unchanged"
    );
}

#[tokio::test]
async fn test_service_status_returns_ok() {
    let service = stub_service();
    let result = service.status().await.expect("status should succeed");

    assert_eq!(
        result.get("status").and_then(|v| v.as_str()),
        Some("ok"),
        "service status should return ok"
    );
}

#[test]
fn test_scaffold_intent_delegates_through_facade() {
    let service = stub_service();
    let result = service
        .scaffold_intent(ScaffoldIntent {
            display_name: "Lab Gateway".into(),
            crate_name: "lab-gateway-mcp".into(),
            binary_name: "lab-gateway".into(),
            server_category: "application platform".into(),
            env_prefix: "lab".into(),
            auth_kind: "api key".into(),
            host: "".into(),
            port: 3100,
            mcp_transport: "streamable-http".into(),
            mcp_primitives: "tools, resources".into(),
            deployment: "containers".into(),
            plugins: "claude".into(),
            publish_mcp: true,
            crawl_urls: "https://docs.synapse2.test".into(),
            crawl_repos: "".into(),
            crawl_search_topics: "Lab API".into(),
        })
        .expect("valid scaffold intent should build through the facade");

    assert_eq!(result["kind"], "synapse2_scaffold_intent");
    assert_eq!(result["project"]["service_name"], "lab_gateway");
}

#[test]
fn test_elicited_name_greeting_transformation_lives_in_service() {
    let service = stub_service();
    let result = service.elicited_name_greeting(ElicitedNameOutcome::Accepted("  Ada  "));

    assert_eq!(result["name"], "Ada");
    assert!(result["greeting"]
        .as_str()
        .expect("greeting should be a string")
        .contains("Ada"));
}

#[test]
fn test_elicited_name_fallback_outcomes_are_covered_below_live_mcporter() {
    let service = stub_service();

    assert_eq!(
        service.elicited_name_greeting(ElicitedNameOutcome::NoInput)["greeting"],
        "Hello! (you provided no name - that's okay)"
    );
    assert_eq!(
        service.elicited_name_greeting(ElicitedNameOutcome::Declined)["greeting"],
        "Hello, anonymous user!"
    );
    assert_eq!(
        service.elicited_name_greeting(ElicitedNameOutcome::Cancelled)["greeting"],
        "Hello there!"
    );
    assert_eq!(
        service.elicited_name_greeting(ElicitedNameOutcome::Unsupported)["fallback_greeting"],
        "Hello, World! (elicitation unavailable)"
    );
}
