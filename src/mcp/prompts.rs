//! MCP prompts for the synapse2 server.
//!
//! Prompts are pre-canned message templates that MCP clients can invoke.
//! They appear in the "Prompts" section of compatible MCP UIs.

use rmcp::model::{
    GetPromptRequestParams, GetPromptResult, ListPromptsResult, Prompt, PromptMessage, Role,
};

pub(super) fn list_prompts() -> ListPromptsResult {
    ListPromptsResult {
        prompts: vec![Prompt::new(
            "quick_start",
            Some(
                "List the configured hosts and check Docker status to verify the \
                 MCP connection is working end-to-end.",
            ),
            None,
        )],
        ..Default::default()
    }
}

pub(super) fn get_prompt(request: GetPromptRequestParams) -> anyhow::Result<GetPromptResult> {
    match request.name.as_str() {
        "quick_start" => Ok(GetPromptResult::new(vec![PromptMessage::new_text(
            Role::User,
            "Use the scout tool with {\"action\":\"nodes\"} to list the configured hosts. \
             Choose one returned host, then use the flux tool with \
             {\"action\":\"host\",\"subaction\":\"status\",\"host\":\"<returned-host>\"} \
             to check that host. Report back both results.",
        )])
        .with_description(
            "Verify the MCP server is working by listing hosts and checking Docker status",
        )),
        other => Err(anyhow::anyhow!("unknown prompt: {other}")),
    }
}

#[cfg(test)]
#[path = "prompts_tests.rs"]
mod tests;
