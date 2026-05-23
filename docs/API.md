# synapse2 API

`synapse2` exposes MCP tools named `flux` and `scout`, one REST action endpoint at `/v1/synapse2`, and equivalent CLI commands.

## MCP Tools

### `flux`

```json
{"action":"docker_info"}
{"action":"container_logs","container":"nginx","tail":100}
{"action":"help"}
```

### `scout`

```json
{"action":"nodes"}
{"action":"exec","command":"hostname"}
{"action":"help"}
```

## CLI Parity

```bash
synapse2 flux docker-info
synapse2 flux container-logs --container nginx --tail 100
synapse2 scout nodes
synapse2 scout exec --command hostname
```

## REST Endpoint

`POST /v1/synapse2`

```json
{
  "action": "scout_nodes",
  "params": {}
}
```

REST is a thin compatibility surface. MCP and CLI are the primary supported surfaces.

## Security Rules

- `help` actions are public.
- Read actions require `synapse2:read`.
- `scout exec` requires `synapse2:write`.
- `synapse2:write` satisfies read.
- Command execution is allowlist-based and rejects traversal/metacharacter patterns.
- Responses are capped before returning to MCP clients.
