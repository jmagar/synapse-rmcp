---
title: "mcporter Integration Testing"
doc_type: "guide"
status: "active"
owner: "synapse2"
audience:
  - "contributors"
  - "agents"
scope: "service"
source_of_truth: false
upstream_refs:
  - "docs/PATTERNS.md"
last_reviewed: "2026-06-13"
---

# mcporter

`mcporter` is used for live MCP integration testing and CLI generation.

## Test harness

The live test script is:

```bash
tests/mcporter/test-mcp.sh
```

Run it through Just:

```bash
just dev
just test-mcporter
```

## Configuration

```json
{
  "mcpServers": {
    "synapse2": {
      "url": "http://localhost:40080/mcp",
      "transport": "http"
    }
  }
}
```

The script targets `http://<SYNAPSE_MCP_HOST>:<SYNAPSE_MCP_PORT>/mcp`, defaulting to `http://localhost:40080/mcp` to match `just dev`. It remaps `0.0.0.0` to `localhost`. If `SYNAPSE_MCP_TOKEN` is set, it sends `Authorization: Bearer <token>`. Credentials are also sourced from `~/.synapse2/.env` when present.

synapse2 exposes two real MCP tools: `flux` (Docker: docker / container / host / compose) and `scout` (SSH/local host inspection: nodes / peek / find / ps / df / delta / exec / emit / beam / zfs / logs). The smoke-test only exercises **read-only / non-destructive** actions — destructive actions (docker build/rmi/prune, container stop/recreate/exec, compose down/restart/recreate, scout exec/emit/beam) require elicitation confirmation and are never called.

## What the test suite validates

- auth rejection when `SYNAPSE_MCP_TOKEN` is set (unauthenticated and bad-token both return HTTP 401)
- `scout` semantic behavior: `nodes` returns a `hosts` array, `help` returns `tool == "scout"` with a `topics` index
- `flux` semantic behavior: `docker info` returns its stable formatted heading, `host status` returns a bounded fanout envelope (`count` and `status` list), and `help` returns `tool == "flux"` with the docker action group
- MCP resource behavior for `synapse://schema/flux` and `synapse://schema/scout` (each returns its matching tool definition)

The host-status checks assert the always-present bounded fanout envelope rather than inner daemon data, so they pass whether or not configured hosts are reachable. Docker info uses its Markdown heading because the full JSON payload can exceed the response cap on multi-host deployments. Tool calls run through mcporter; resource reads use JSON-RPC directly because mcporter 0.12 does not accept ad-hoc HTTP resource URLs. Bearer-auth tool calls fall back to JSON-RPC `tools/call` when an older mcporter does not support HTTP headers on `mcporter call`.

## Test philosophy

Use semantic assertions, not liveness-only checks:

```bash
# Bad test — only proves MCP responded
run_test "scout nodes" "scout" '{"action":"nodes","response_format":"json"}'

# Good test — proves the service actually returned the right shape
run_test "scout nodes returns hosts" "scout" '{"action":"nodes","response_format":"json"}' "hosts" "array"
run_test_semantic "flux help tool name" "flux" '{"action":"help","format":"json"}' "tool" "flux" "exact"
```

A test that checks `is_error: false` is not a good test — it only verifies the MCP protocol layer responded. Semantic tests check that the actual service data is present and structurally correct.

## Tool validation helpers

```bash
# Validate that a JSON path exists and is non-empty
assert_key() {
  local label="$1" output="$2" key_path="$3"
  python3 -c "
import sys, json
d = json.loads('''${output}''')
keys = '${key_path}'.split('.')
node = d
for k in keys:
    node = node[int(k)] if isinstance(node, list) and k.isdigit() else node[k]
assert node is not None and node != '' and node != [] and node != {}
" 2>/dev/null || { echo \"[FAIL] ${label}: missing or empty .${key_path}\"; return 1; }
}
```

## Resource validation

MCP resources are public contract, not implementation detail. Test every stable resource URI. For the schema resources (`synapse://schema/flux` and `synapse://schema/scout`, each of which returns its matching tool definition):

- The resource URI resolves.
- The returned content parses as one JSON object.
- The tool definition name matches the resource URI (`flux` or `scout`).
- The tool's `inputSchema.type` is `object`.
- The tool's `inputSchema.properties.action` exists.

## Generated CLIs

`just generate-cli` demonstrates generating a standalone CLI from a running MCP server. Generated CLIs may embed auth material; do not commit them unless they are intentionally scrubbed and reviewed.

See `docs/PATTERNS.md` §17 for the full mcporter integration test pattern.
