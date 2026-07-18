use super::{reporter::PatternReporter, util::read_file};

const ACTION_TEST_COVERAGE_EXCEPTIONS: &[&str] = &[
    // Requires a live MCP Peer<RoleServer>; covered by parser/schema/help checks instead.
    "elicit_name",
];

pub(super) fn action_surfaces(reporter: &mut PatternReporter) {
    let actions_text = read_file("src/actions/operations.rs");
    let action_specs = operation_specs_body(&actions_text).unwrap_or(&actions_text);
    let action_names = extract_action_names(action_specs);

    if action_names.is_empty() {
        reporter.fail(
            "actions",
            "could not parse OPERATION_SPECS from src/actions/operations.rs",
        );
        return;
    }

    let schema = read_file("src/mcp/schemas.rs");
    let help = read_file("src/mcp/help.rs");
    let tests = read_file("tests/tool_dispatch.rs");
    let cli = read_file("src/cli.rs");

    let schema_uses_metadata = schema.contains("action_names()");
    let missing_schema = if schema_uses_metadata {
        Vec::new()
    } else {
        action_names
            .iter()
            .filter(|action| !schema.contains(&format!("\"{action}\"")))
            .cloned()
            .collect::<Vec<_>>()
    };
    let missing_help = action_names
        .iter()
        .filter(|action| !has_help_entry(&help, action))
        .cloned()
        .collect::<Vec<_>>();
    let missing_tests = action_names
        .iter()
        .filter(|action| {
            action.as_str() != "help"
                && !ACTION_TEST_COVERAGE_EXCEPTIONS.contains(&action.as_str())
                && !tests.contains(action.as_str())
        })
        .cloned()
        .collect::<Vec<_>>();
    let missing_cli = action_names
        .iter()
        .filter(|action| action.as_str() != "help")
        .filter(|action| {
            !cli.contains(&format!("\"{action}\"")) && !cli.contains(&variant_name(action))
        })
        .cloned()
        .collect::<Vec<_>>();

    if !missing_schema.is_empty() {
        reporter.fail(
            "actions",
            format!(
                "schemas.rs missing action(s): {}",
                missing_schema.join(", ")
            ),
        );
    }
    if !missing_help.is_empty() {
        reporter.fail(
            "actions",
            format!(
                "src/mcp/help.rs missing action help entry/entries: {}. Hint: add a topic-aware help entry or legacy help listing.",
                missing_help.join(", ")
            ),
        );
    }
    if !missing_tests.is_empty() {
        reporter.warn(
            "actions",
            format!(
                "tests/tool_dispatch.rs may be missing action coverage: {}. Hint: add a direct dispatch/service test or an explicit exception.",
                missing_tests.join(", ")
            ),
        );
    }
    if !missing_cli.is_empty() {
        reporter.warn(
            "cli-mcp-parity",
            format!(
                "CLI may be missing non-MCP-only action(s): {}. Hint: add a Command variant, parse arm, and dispatch arm.",
                missing_cli.join(", ")
            ),
        );
    }
    if missing_schema.is_empty()
        && missing_help.is_empty()
        && missing_tests.is_empty()
        && missing_cli.is_empty()
    {
        reporter.ok(
            "actions",
            format!(
                "{} actions appear in schema/help/tests/CLI surfaces",
                action_names.len()
            ),
        );
    }
}

fn has_help_entry(help: &str, action: &str) -> bool {
    help.contains(&format!("\"{action}\""))
        || help.contains(&format!("m.insert(\"{action}\""))
        || help.contains(&format!("m.insert(\"{action}:"))
}

fn operation_specs_body(text: &str) -> Option<&str> {
    let start = text.find("OPERATION_SPECS")?;
    let after_start = &text[start..];
    let end = after_start.find("];")?;
    Some(&after_start[..end])
}

fn extract_action_names(text: &str) -> Vec<String> {
    let mut actions = Vec::new();
    for line in text.lines().filter(|line| line.contains("operation!(")) {
        let quoted = line.split('"').collect::<Vec<_>>();
        let Some(action) = quoted.get(3) else {
            continue;
        };
        let action = (*action).to_owned();
        if !actions.contains(&action) {
            actions.push(action);
        }
    }
    actions
}

fn variant_name(action: &str) -> String {
    action
        .split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().chain(chars).collect::<String>(),
                None => String::new(),
            }
        })
        .collect::<String>()
}

#[cfg(test)]
mod tests {
    use super::*;

    const ACTIONS: &str = r#"
pub const OPERATION_SPECS: &[OperationSpec] = &[
    operation!("flux.greet", Flux, "greet", None, Some(READ_SCOPE), false, Rest, []),
    operation!("scout.elicit_name", Scout, "elicit_name", None, Some(READ_SCOPE), false, McpOnly, []),
    operation!("scout.greet", Scout, "greet", None, Some(READ_SCOPE), false, McpOnly, []),
];

pub fn rest_help() {
    let example = "Alice";
}
"#;

    #[test]
    fn operation_specs_body_limits_parsing_to_metadata_block() {
        let body = operation_specs_body(ACTIONS).expect("OPERATION_SPECS body should parse");
        assert!(body.contains("greet"));
        assert!(!body.contains("Alice"));
    }

    #[test]
    fn action_name_parser_ignores_non_metadata_names() {
        let body = operation_specs_body(ACTIONS).unwrap();
        assert_eq!(extract_action_names(body), vec!["greet", "elicit_name"]);
    }

    #[test]
    fn variant_name_matches_cli_enum_style() {
        assert_eq!(variant_name("elicit_name"), "ElicitName");
    }
}
