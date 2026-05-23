//! Tool JSON schemas for the MCP synapse2 tool.
//!
//! This file defines the action list and input schema for the `synapse2` tool.
//! MCP clients inspect this schema to know what arguments are valid.
//!
//! **Template**: rename `synapse2` to your tool name. Add/remove actions and
//! parameters to match your service. Use `"required": [...]` for mandatory args.

use std::sync::OnceLock;

use serde_json::{json, Value};

/// Cached JSON schema definitions (static data, built once at first call).
static TOOL_DEFINITIONS: OnceLock<Vec<Value>> = OnceLock::new();

/// Return the JSON schema definitions for all tools (cached after first call).
///
/// Returns a `Vec<Value>` where each item is a tool definition object matching
/// the MCP `Tool` schema: `{ name, description, inputSchema }`.
///
/// This is also used by the schema resource (`synapse://schema/mcp-tool`).
pub(super) fn tool_definitions() -> &'static Vec<Value> {
    TOOL_DEFINITIONS.get_or_init(build_tool_definitions)
}

fn build_tool_definitions() -> Vec<Value> {
    vec![
        json!({
            "name": "flux",
            "description": "Docker infrastructure management for synapse2. First slice supports read-only docker, container, and host status actions.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "action": { "type": "string", "enum": ["help", "docker", "container", "host"] },
                    "subaction": { "type": "string" },
                    "host": { "type": "string" },
                    "container_id": { "type": "string" },
                    "lines": { "type": "integer", "minimum": 1, "maximum": 500 }
                },
                "required": ["action"],
                "additionalProperties": false
            }
        }),
        json!({
            "name": "scout",
            "description": "SSH/local host inspection for synapse2. First slice supports nodes, peek, and allowlisted exec.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "action": { "type": "string", "enum": ["help", "nodes", "peek", "exec"] },
                    "host": { "type": "string" },
                    "path": { "type": "string" },
                    "command": { "type": "string" }
                },
                "required": ["action"],
                "additionalProperties": false
            }
        }),
    ]
}

#[cfg(test)]
#[path = "schemas_tests.rs"]
mod tests;
