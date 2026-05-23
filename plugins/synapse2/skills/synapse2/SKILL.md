---
name: synapse2
description: Use when the user wants to inspect local Docker/host state or run guarded Synapse scout operations through synapse2.
---

# synapse2

Use `flux` for read-only Docker/host inspection and `scout` for node discovery or guarded command execution.

## Common Calls

```text
mcp__synapse2__flux(action="docker_info")
mcp__synapse2__flux(action="container_logs", container="nginx", tail=100)
mcp__synapse2__scout(action="nodes")
mcp__synapse2__scout(action="exec", command="hostname")
```

## Tool Guidance

| Tool | Action | Use When |
|---|---|---|
| `flux` | `docker_info` | The user asks whether Docker is available or wants host Docker metadata |
| `flux` | `container_logs` | The user asks for recent logs from a named container |
| `scout` | `nodes` | The user asks what nodes are visible |
| `scout` | `exec` | The user asks to run an allowlisted diagnostic command |

## Safety

- Prefer `flux` read actions before `scout exec`.
- Do not try to bypass the command allowlist.
- Do not pass shell pipelines, redirections, traversal, or compound commands.
- Summarize command output when it is large.
