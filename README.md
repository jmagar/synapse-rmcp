# synapse2

Rust MCP and CLI server for local Synapse workflows.

`synapse2` is a Rust implementation of the existing `synapse-mcp` behavior. It exposes two MCP tools with matching CLI commands:

- `flux`: operational Docker/host inspection helpers
- `scout`: guarded command execution and node discovery helpers

## Surfaces

| Surface | Status | Purpose |
|---|---:|---|
| MCP | Required | Agent-facing `flux` and `scout` tools |
| CLI | Required | Scriptable parity surface |
| REST | Present | Thin local action endpoint from the template |
| Web | Present | Lightweight template admin shell, not the primary surface |

## Tools and Actions

### `flux`

| Action | Scope | Description |
|---|---|---|
| `docker_info` | `synapse2:read` | Return Docker availability and basic host information |
| `container_logs` | `synapse2:read` | Return bounded logs for a container |
| `help` | public | Return flux action reference |

### `scout`

| Action | Scope | Description |
|---|---|---|
| `nodes` | `synapse2:read` | Return known/local node information |
| `exec` | `synapse2:write` | Run an allowlisted command with guardrails |
| `help` | public | Return scout action reference |

## Configuration

```bash
SYNAPSE2_MCP_HOST=127.0.0.1
SYNAPSE2_MCP_PORT=3100
SYNAPSE2_MCP_TOKEN=change-me
SYNAPSE2_SCOUT_ALLOWED_COMMANDS=docker,hostname,uptime,whoami
SYNAPSE2_SCOUT_DENIED_PATTERNS=..,;,&&,|,>,<
```

`scout exec` rejects path traversal and shell metacharacter patterns before execution.

## Run

```bash
cargo run -- flux docker-info
cargo run -- flux container-logs --container nginx --tail 100
cargo run -- scout nodes
cargo run -- scout exec --command hostname

cargo run -- serve
cargo run -- mcp
```

MCP examples:

```json
{"name":"flux","arguments":{"action":"docker_info"}}
{"name":"scout","arguments":{"action":"nodes"}}
{"name":"scout","arguments":{"action":"exec","command":"hostname"}}
```

## Architecture

```text
Docker helpers  (src/docker.rs)    host/Docker inspection
Synapse client  (src/synapse.rs)   scout guardrails and execution
      ↓
Service layer   (src/app.rs)       validation and response shaping
      ↓
MCP shims       (src/mcp/tools.rs) tool args -> service -> Value
CLI shim        (src/cli.rs)       argv -> service -> stdout
```

## Development

```bash
cargo fmt --check
cargo test
cargo clippy -- -D warnings
cargo build --release
```

Useful docs:

- `docs/API.md` for tool contracts
- `docs/CONFIG.md` for environment and auth
- `docs/QUICKSTART.md` for local smoke tests
- `docs/MCP_SCHEMA.md` for schema drift rules
- `plugins/synapse2/skills/synapse2/SKILL.md` for agent usage guidance
