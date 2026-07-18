//! REST projection of the canonical operation registry and request conversion.

use anyhow::Result;
use serde_json::{Map, Value, json};

use super::{
    OPERATION_SPECS, OperationSpec, OperationTool, OperationTransport, SynapseAction,
    ValidationError,
};

pub fn operation(name: &str) -> Option<&'static OperationSpec> {
    OPERATION_SPECS
        .iter()
        .find(|spec| spec.name == name && spec.transport == OperationTransport::Rest)
}

pub fn operations() -> impl Iterator<Item = &'static OperationSpec> {
    OPERATION_SPECS
        .iter()
        .filter(|spec| spec.transport == OperationTransport::Rest)
}

pub fn action_names() -> Vec<&'static str> {
    operations().map(|spec| spec.name).collect()
}

pub fn action_from_request(name: &str, params: &Value) -> Result<SynapseAction> {
    action_and_spec_from_request(name, params).map(|(action, _)| action)
}

pub fn action_and_spec_from_request(
    name: &str,
    params: &Value,
) -> Result<(SynapseAction, &'static OperationSpec)> {
    let spec = operation(name).ok_or_else(|| ValidationError::UnknownAction {
        action: name.to_owned(),
    })?;
    let mut args: Map<String, Value> = match params {
        Value::Null => Map::new(),
        Value::Object(map) => map.clone(),
        _ => {
            return Err(ValidationError::WrongType {
                field: "params".into(),
            }
            .into());
        }
    };
    args.insert("action".into(), json!(spec.action));
    if let Some(subaction) = spec.subaction {
        args.insert("subaction".into(), json!(subaction));
    }
    let args = Value::Object(args);
    let action = match spec.tool {
        OperationTool::Flux | OperationTool::Both => SynapseAction::from_flux_args(&args),
        OperationTool::Scout => SynapseAction::from_scout_args(&args),
    }?;
    Ok((action, spec))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_unique_and_parseable_with_required_placeholders() {
        let mut names = std::collections::HashSet::new();
        for spec in operations() {
            assert!(names.insert(spec.name), "duplicate {}", spec.name);
            let mut params = Map::new();
            for required in spec.required_params {
                let value = if *required == "force" {
                    json!(true)
                } else {
                    json!("value")
                };
                params.insert((*required).into(), value);
            }
            let action = action_from_request(spec.name, &Value::Object(params))
                .unwrap_or_else(|error| panic!("{}: {error}", spec.name));
            assert_eq!(
                spec.required_scope,
                super::super::required_scope_for_parsed_action(&action),
                "{} scope drift",
                spec.name
            );
        }
    }

    #[test]
    fn params_must_be_an_object() {
        assert!(action_from_request("help", &json!([])).is_err());
    }

    #[test]
    fn mcp_only_metadata_is_the_operation_level_rest_set_difference() {
        let mcp_only = super::super::mcp_only_action_names();
        assert!(mcp_only.contains(&"flux.container.inspect"));
        assert!(mcp_only.contains(&"flux.host.status"));
        assert!(mcp_only.contains(&"scout.find"));
        assert!(!mcp_only.contains(&"flux.container.list"));
        assert!(!mcp_only.contains(&"scout.peek"));
    }
}
