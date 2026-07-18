use super::{flux_operation_branches, scout_operation_branches, tool_definitions};

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

#[test]
fn flux_schema_includes_host_parser_fields() {
    let tools = tool_definitions();
    let flux = tools
        .iter()
        .find(|tool| tool["name"] == "flux")
        .expect("flux schema should exist");
    let props = &flux["inputSchema"]["properties"];

    for field in ["protocol", "offset", "checks"] {
        assert!(
            props[field].is_object(),
            "flux schema should expose parser-supported field {field}"
        );
    }

    let host_description = props["host"]["description"].as_str().unwrap_or_default();
    assert!(host_description.contains("host services/mounts/ports/doctor"));
    assert!(host_description.contains("compose ops including list"));
}

#[test]
fn conditional_branches_cover_complete_operation_inventory() {
    let mut operations = Vec::new();
    for (tool, branches) in [
        ("flux", flux_operation_branches()),
        ("scout", scout_operation_branches()),
    ] {
        for branch in branches {
            let props = &branch["properties"];
            let action = props["action"]["const"].as_str().unwrap();
            if action == "help" {
                operations.push("help".to_owned());
            } else if let Some(subaction) = props["subaction"]["const"].as_str() {
                operations.push(format!("{tool}.{action}.{subaction}"));
            } else {
                operations.push(format!("{tool}.{action}"));
            }
        }
    }
    operations.sort();
    operations.dedup();
    let mut expected = crate::actions::mcp_operation_names()
        .into_iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    expected.sort();
    assert_eq!(operations, expected);
}

#[test]
fn conditional_branches_require_subactions_and_operation_fields() {
    let branches = flux_operation_branches();
    let find = |action: &str, subaction: &str| {
        branches
            .iter()
            .find(|branch| {
                branch["properties"]["action"]["const"] == action
                    && branch["properties"]["subaction"]["const"] == subaction
            })
            .unwrap()
    };
    assert!(
        find("container", "list")["required"]
            .as_array()
            .unwrap()
            .contains(&"subaction".into())
    );
    for field in ["host", "container_id", "command"] {
        assert!(
            find("container", "exec")["required"]
                .as_array()
                .unwrap()
                .contains(&field.into())
        );
    }
}

#[test]
fn scout_delta_schema_requires_one_complete_target_shape() {
    let delta = scout_operation_branches()
        .into_iter()
        .find(|branch| branch["properties"]["action"]["const"] == "delta")
        .unwrap();
    assert_eq!(
        delta["oneOf"][0]["required"],
        serde_json::json!(["content"])
    );
    assert_eq!(
        delta["oneOf"][1]["required"],
        serde_json::json!(["target_host", "target_path"])
    );
}
