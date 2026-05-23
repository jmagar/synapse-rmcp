use serde_json::json;

use crate::{mcp::execute_tool_without_peer_for_test, testing::loopback_state};

#[tokio::test]
async fn dispatches_flux_and_scout_tools() {
    let state = loopback_state();
    let flux = execute_tool_without_peer_for_test(&state, "flux", json!({"action":"help"}))
        .await
        .unwrap();
    assert_eq!(flux["tool"], "flux");

    let scout = execute_tool_without_peer_for_test(&state, "scout", json!({"action":"nodes"}))
        .await
        .unwrap();
    assert!(scout["hosts"].is_array());
}

#[tokio::test]
async fn rejects_unknown_tool() {
    let state = loopback_state();
    let error = execute_tool_without_peer_for_test(&state, "missing", json!({}))
        .await
        .unwrap_err();
    assert!(error.to_string().contains("unknown tool"));
}
