use serde_json::json;
use synapse2::{mcp::execute_tool_without_peer_for_test, testing::loopback_state};

async fn call_mcp_tool(tool: &str, args: serde_json::Value) -> serde_json::Value {
    let state = loopback_state();
    execute_tool_without_peer_for_test(&state, tool, args)
        .await
        .expect("MCP tool dispatch should succeed")
}

async fn call_mcp_tool_error(tool: &str, args: serde_json::Value) -> String {
    let state = loopback_state();
    execute_tool_without_peer_for_test(&state, tool, args)
        .await
        .expect_err("MCP tool dispatch should fail")
        .to_string()
}

#[tokio::test]
async fn flux_help_returns_action_reference() {
    let result = call_mcp_tool("flux", json!({ "action": "help" })).await;
    assert_eq!(result["tool"], "flux");
    assert!(result["actions"]["docker"].is_array());
}

#[tokio::test]
async fn topic_help_dispatches_for_flux_and_scout() {
    let flux = call_mcp_tool(
        "flux",
        json!({ "action": "help", "topic": "container:list", "format": "json" }),
    )
    .await;
    assert_eq!(flux["topic"], "container:list");
    assert!(!flux["text"].as_str().unwrap().is_empty());

    let scout = call_mcp_tool(
        "scout",
        json!({ "action": "help", "topic": "logs:journal", "format": "json" }),
    )
    .await;
    assert_eq!(scout["topic"], "logs:journal");
    assert!(!scout["text"].as_str().unwrap().is_empty());
}

#[tokio::test]
async fn flux_docker_info_is_safe_without_docker() {
    // docker info now fans out across configured hosts via bollard (B10). With
    // no reachable daemon the per-host op errors, but the aggregate shape is
    // always present and the call never panics.
    let result = call_mcp_tool("flux", json!({ "action": "docker", "subaction": "info" })).await;
    assert!(result.get("count").is_some());
    assert!(result.get("info").is_some());
    assert!(result.get("partial").is_some());
}

#[tokio::test]
async fn flux_read_only_families_dispatch_with_parameters() {
    let containers = call_mcp_tool(
        "flux",
        json!({
            "action": "container",
            "subaction": "list",
            "host": "local",
            "state": "all",
            "name_filter": "synapse",
            "image_filter": "rust",
            "label_filter": "com.example=true",
            "response_format": "json"
        }),
    )
    .await;
    assert!(containers["containers"].is_array());
    assert!(containers.get("partial").is_some());

    let host = call_mcp_tool(
        "flux",
        json!({ "action": "host", "subaction": "info", "host": "local" }),
    )
    .await;
    assert!(host.get("count").is_some());
    assert!(host.get("info").is_some());
    assert!(host.get("partial").is_some());

    let compose = call_mcp_tool(
        "flux",
        json!({ "action": "compose", "subaction": "refresh", "host": "local" }),
    )
    .await;
    assert_eq!(compose, json!({ "host": "local", "refreshed": true }));
}

#[tokio::test]
async fn scout_nodes_returns_hosts_array() {
    let result = call_mcp_tool("scout", json!({ "action": "nodes" })).await;
    assert!(result["hosts"].is_array());
}

#[tokio::test]
async fn scout_read_only_families_dispatch_with_parameters() {
    let tempdir = tempfile::tempdir_in("/tmp").expect("tempdir should be created under /tmp");
    let path = tempdir.path().join("synapse2-tool-dispatch.txt");
    std::fs::write(&path, "alpha\nbeta\n").expect("test fixture should be writable");
    let path = path.to_string_lossy().to_string();
    let dir = tempdir.path().to_string_lossy().to_string();

    let peek = call_mcp_tool(
        "scout",
        json!({ "action": "peek", "host": "local", "path": path, "tree": false, "depth": 99 }),
    )
    .await;
    assert_eq!(peek["host"], "local");
    assert_eq!(peek["kind"], "file");
    assert_eq!(peek["content"], "alpha\nbeta\n");

    let find = call_mcp_tool(
        "scout",
        json!({
            "action": "find",
            "host": "local",
            "path": dir,
            "pattern": "synapse2-tool-dispatch.txt",
            "depth": 99,
            "limit": 1
        }),
    )
    .await;
    assert_eq!(find["count"], 1);
    assert_eq!(find["files"][0], path);

    let delta = call_mcp_tool(
        "scout",
        json!({
            "action": "delta",
            "source_host": "local",
            "source_path": path,
            "content": "alpha\ngamma\n"
        }),
    )
    .await;
    assert_eq!(delta["target"], "inline");
    assert_eq!(delta["identical"], false);
    assert!(delta["diff"].as_str().unwrap().contains("gamma"));

    let ps = call_mcp_tool(
        "scout",
        json!({ "action": "ps", "host": "local", "sort": "pid", "limit": 1 }),
    )
    .await;
    assert_eq!(ps["host"], "local");
    assert_eq!(ps["sort"], "pid");
    assert!(ps["rows"].is_array());

    let df = call_mcp_tool(
        "scout",
        json!({ "action": "df", "host": "local", "path": "/tmp" }),
    )
    .await;
    assert_eq!(df["host"], "local");
    assert_eq!(df["path"], "/tmp");
    assert!(df["disk_usage"].as_str().unwrap().contains("Filesystem"));
}

#[tokio::test]
async fn scout_exec_rejects_denied_commands() {
    let state = loopback_state();
    let error = execute_tool_without_peer_for_test(
        &state,
        "scout",
        json!({ "action": "exec", "host": "local", "path": "/tmp", "command": "rm" }),
    )
    .await
    .expect_err("denied command should fail");
    assert!(error.to_string().contains("denied"));
}

#[tokio::test]
async fn destructive_actions_are_confirmation_gated_before_io() {
    let scout_error = call_mcp_tool_error(
        "scout",
        json!({
            "action": "exec",
            "host": "local",
            "path": "/tmp",
            "command": "whoami",
            "args": []
        }),
    )
    .await;
    assert!(scout_error.contains("requires confirmation"));

    let flux_error = call_mcp_tool_error(
        "flux",
        json!({
            "action": "docker",
            "subaction": "prune",
            "host": "local",
            "prune_target": "containers",
            "force": true
        }),
    )
    .await;
    assert!(flux_error.contains("requires confirmation"));

    let emit_error = call_mcp_tool_error(
        "scout",
        json!({
            "action": "emit",
            "targets": [{ "host": "local", "path": "/tmp" }],
            "command": "whoami",
            "args": []
        }),
    )
    .await;
    assert!(emit_error.contains("requires confirmation"));

    let beam_error = call_mcp_tool_error(
        "scout",
        json!({
            "action": "beam",
            "source_host": "local",
            "source_path": "/tmp/source.txt",
            "dest_host": "local",
            "dest_path": "/tmp/dest.txt"
        }),
    )
    .await;
    assert!(beam_error.contains("requires confirmation"));
}

#[tokio::test]
async fn dispatch_validation_covers_missing_wrong_type_and_unknown_subactions() {
    let missing_query = call_mcp_tool_error(
        "flux",
        json!({ "action": "container", "subaction": "search" }),
    )
    .await;
    assert!(missing_query.contains("`query` is required"));

    let wrong_bool = call_mcp_tool_error(
        "flux",
        json!({ "action": "docker", "subaction": "images", "dangling_only": "yes" }),
    )
    .await;
    assert!(wrong_bool.contains("`dangling_only` must be"));

    let wrong_array = call_mcp_tool_error(
        "scout",
        json!({
            "action": "exec",
            "host": "local",
            "path": "/tmp",
            "command": "whoami",
            "args": ["--version", 1]
        }),
    )
    .await;
    assert!(wrong_array.contains("`args[]` must be a string"));

    let unknown_zfs = call_mcp_tool_error(
        "scout",
        json!({ "action": "zfs", "subaction": "bogus", "host": "local" }),
    )
    .await;
    assert!(unknown_zfs.contains("unknown zfs subaction"));
}

#[tokio::test]
async fn unknown_tool_and_missing_action_are_rejected() {
    let state = loopback_state();
    assert!(
        execute_tool_without_peer_for_test(&state, "missing", json!({}))
            .await
            .is_err()
    );
    let error = execute_tool_without_peer_for_test(&state, "flux", json!({}))
        .await
        .unwrap_err();
    assert!(error.to_string().contains("action is required"));
}
