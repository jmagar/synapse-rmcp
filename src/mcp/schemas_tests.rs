use super::tool_definitions;

#[test]
fn defines_flux_and_scout_tools() {
    let tools = tool_definitions();
    let names = tools
        .iter()
        .map(|tool| tool["name"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(names, vec!["flux", "scout"]);
}

#[test]
fn schemas_disallow_unknown_top_level_properties() {
    for tool in tool_definitions() {
        assert_eq!(tool["inputSchema"]["additionalProperties"], false);
        assert!(tool["inputSchema"]["properties"]["action"]["enum"].is_array());
    }
}
