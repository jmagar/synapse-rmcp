# synapse2 MCP Schema Contract

`synapse2` exposes two MCP tools: `flux` and `scout`.

## Tools

| Tool | Dispatch parameter | Purpose |
|---|---|---|
| `flux` | `action` | Docker and host inspection helpers |
| `scout` | `action` | Node discovery and guarded command execution |

## Actions

| Tool | Action | Scope | Description |
|---|---|---|---|
| `flux` | `docker_info` | `synapse2:read` | Return Docker availability and host information |
| `flux` | `container_logs` | `synapse2:read` | Return bounded container logs |
| `flux` | `help` | public | Return flux action reference |
| `scout` | `nodes` | `synapse2:read` | Return known/local node information |
| `scout` | `exec` | `synapse2:write` | Run an allowlisted command with guardrails |
| `scout` | `help` | public | Return scout action reference |

## Drift Rules

- `src/actions.rs` is the source of truth for action parsing and scopes.
- `src/mcp/schemas.rs` must reject unknown top-level parameters.
- `README.md`, `docs/API.md`, and `plugins/synapse2/skills/synapse2/SKILL.md` must mention each shipped tool.
- `scout exec` must stay allowlist-based and reject traversal/metacharacter patterns.
