# synapse-rmcp

Rust MCP and CLI server for local Synapse workflows — a full-parity port of
[synapse-mcp](https://github.com/jmagar/synapse-mcp) implemented in Rust with
the [rmcp](https://github.com/modelcontextprotocol/rust-sdk) framework.

The server exposes two MCP tools (`flux` and `scout`) plus equivalent CLI
commands, covering all 59 production actions from the original TypeScript server.

## Contents

- [Naming](#naming)
- [Capabilities And Boundaries](#capabilities-and-boundaries)
- [Install](#install)
- [Quickstart](#quickstart)
- [Client Configuration](#client-configuration)
- [Runtime Surfaces](#runtime-surfaces)
- [MCP Tool Reference](#mcp-tool-reference)
- [CLI Reference](#cli-reference)
- [Configuration](#configuration)
- [Authentication](#authentication)
- [Safety And Trust Model](#safety-and-trust-model)
- [Architecture](#architecture)
- [Distribution Contract](#distribution-contract)
- [Development](#development)
- [Verification](#verification)
- [Deployment](#deployment)
- [Troubleshooting](#troubleshooting)
- [Documentation](#documentation)
- [Related Servers](#related-servers)
- [License](#license)

## Naming

The repository is `synapse-rmcp`, the Rust crate is `synapse2`, the MCP server
identity is `synapse2`, and the installed binary is `synapse`. The npm launcher
package is `synapse-rmcp`.

Across most of the RMCP family, naming follows
`repo=<service>-rmcp`, `npm=<service>-rmcp`, and `CLI=r<service>`. Synapse is an
exception because it is a Rust port of the older TypeScript `synapse-mcp`
project and keeps the operator-facing `synapse` binary.

## Capabilities And Boundaries

Synapse provides local Docker, Compose, host, SSH, log, ZFS, and file-operation
workflows through two MCP tools and the equivalent CLI:

- `flux` manages Docker infrastructure, containers, Compose projects, and host
  inspection.
- `scout` handles SSH/local host inspection, safe file reads, allowlisted
  command execution, ZFS introspection, and log retrieval.
- REST exists only as a compatibility shim for a subset of actions.
- The web surface is a lightweight static admin shell, not a full dashboard.

**Not for:** arbitrary shell access, unaudited remote mutation, or bypassing
host SSH trust. Destructive Docker/Compose/exec/file-transfer actions require
explicit host targets and confirmation policy.

MCP callers never provide credentials, tokens, keys, or secrets as action
arguments. Tokens, OAuth settings, host topology, SSH trust, and allowlists live
in server-side configuration or the local SSH environment.

## Install

Use the npm launcher for stdio MCP or CLI access without a manual binary
install:

```bash
npx -y synapse-rmcp --help
npx -y synapse-rmcp mcp
```

For a permanent command:

```bash
npm i -g synapse-rmcp
synapse --version
```

From source:

```bash
cargo build --release
```

## Quickstart

The first-screen 30-second path is:

```bash
npx -y synapse-rmcp mcp
```

Then configure an MCP client with stdio:

```json
{
  "mcpServers": {
    "synapse": {
      "command": "npx",
      "args": ["-y", "synapse-rmcp", "mcp"]
    }
  }
}
```

Start with a read-only call:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "scout",
    "arguments": {
      "action": "nodes"
    }
  }
}
```

## Client Configuration

stdio is preferred for local MCP clients:

```json
{
  "mcpServers": {
    "synapse": {
      "command": "synapse",
      "args": ["mcp"]
    }
  }
}
```

Streamable HTTP uses `/mcp` on the configured host and port:

```json
{
  "mcpServers": {
    "synapse": {
      "url": "http://127.0.0.1:40080/mcp",
      "headers": {
        "Authorization": "Bearer ${SYNAPSE_MCP_TOKEN}"
      }
    }
  }
}
```

## Runtime Surfaces

| Surface | Status | Purpose |
|---|---:|---|
| MCP | Required | Agent-facing `flux` and `scout` tools |
| CLI | Required | Scriptable parity surface for operators |
| REST | Compatibility | Thin local action endpoint for 14 of 59 actions |
| Web | Present | Lightweight static admin shell |

## MCP Tool Reference

Synapse exposes two MCP tools:

- `flux`: Docker daemon, container, host, and Compose operations.
- `scout`: SSH/local host inspection, filesystem reads, allowlisted exec,
  multi-host emit, file beam, ZFS, and logs.

Both tools use an action-dispatched JSON shape:

```json
{"name":"flux","arguments":{"action":"docker","subaction":"info"}}
{"name":"scout","arguments":{"action":"nodes"}}
```

The detailed action tables below are curated README reference material. The
generated runtime MCP schema and the Rust action definitions are the source of
truth for the live tool contract.

## CLI Reference

Common commands:

```bash
synapse flux docker info
synapse flux container list
synapse flux compose list --host myhost
synapse scout nodes
synapse scout exec --host myhost --command hostname
synapse scout zfs pools --host myhost
synapse scout logs journal --host myhost --unit docker
```

## Authentication

Mounted HTTP supports bearer and Google OAuth modes. Loopback stdio/local dev
can run without mounted HTTP auth. Important variables:

```bash
SYNAPSE_MCP_HOST=127.0.0.1
SYNAPSE_MCP_PORT=40080
SYNAPSE_MCP_TOKEN=change-me
SYNAPSE_MCP_AUTH_MODE=bearer
```

See [Configuration](#configuration) and `docs/CONFIG.md` for the full auth
matrix.

## Safety And Trust Model

Synapse separates read and write scopes (`synapse:read`, `synapse:write`) and
uses confirmation gates for destructive operations. `SYNAPSE_MCP_ALLOW_DESTRUCTIVE`
can skip prompts only in loopback-safe contexts. SSH host trust is delegated to
OpenSSH known-hosts behavior, and command execution uses execvp/argv semantics
without shell interpolation.

## Distribution Contract

The source of truth for release identity is the version shared by `Cargo.toml`,
`.release-please-manifest.json`, `packages/synapse-rmcp/package.json`, release
artifacts, and `server.json`.

Distribution/version invariants:

- The npm package downloads the matching GitHub Release binary.
- The installed binary remains `synapse`.
- `server.json` must point at `ghcr.io/jmagar/synapse:<version>`.
- Plugin manifests stay versionless where marketplaces derive identity from git
  state.
- Generated docs and schemas must come from source-controlled generation
  inputs; curated README/docs should point at source-of-truth files for details.

## Verification

```bash
python3 /home/jmagar/workspace/soma/scripts/check-readme-guide.py README.md
npm --prefix packages/synapse-rmcp run check
cargo fmt --check
cargo check
cargo test
git diff --check
```

## Deployment

For a persistent mounted HTTP server:

```bash
SYNAPSE_MCP_HOST=0.0.0.0 \
SYNAPSE_MCP_PORT=40080 \
SYNAPSE_MCP_TOKEN=change-me \
synapse serve
```

Use bearer or OAuth before exposing the endpoint beyond loopback. Production
Docker/Compose notes live in `docs/DEPLOYMENT.md` and `docs/DOCKER.md` when
present; local operator usage can stay on stdio.

## Troubleshooting

- `401` or `403` from `/mcp`: check bearer/OAuth settings and gateway headers.
- no hosts appear: check `SYNAPSE_HOSTS_CONFIG`, `SYNAPSE_CONFIG_FILE`, and
  `~/.ssh/config`.
- destructive actions are refused: confirm the host target and confirmation
  policy.
- SSH errors: verify OpenSSH known_hosts, mux/socket availability, and host
  reachability outside Synapse first.

## npm / npx

Run the stdio MCP server or CLI without a manual binary install:

```bash
npx -y synapse-rmcp --help
```

MCP clients can use the same launcher:

```json
{
  "mcpServers": {
    "synapse": {
      "command": "npx",
      "args": ["-y", "synapse-rmcp"]
    }
  }
}
```

The npm package downloads the `synapse` binary from GitHub Releases during `postinstall` and keeps the release tag aligned with `packages/synapse-rmcp/package.json`.

Across the rmcp family, naming follows `repo=<service>-rmcp`, `npm=<service>-rmcp`, and `CLI=r<service>`. Synapse is the exception: the npm package is `synapse-rmcp`, but the installed CLI and binary alias remain `synapse`.

## Surfaces

| Surface | Status | Purpose |
|---|---:|---|
| MCP | Required | Agent-facing `flux` and `scout` tools |
| CLI | Required | Scriptable parity surface |
| REST | Present | Thin local action endpoint |
| Web | Present | Lightweight static admin shell |

## Tools and Actions

### `flux` — Docker infrastructure management

#### `flux docker` — Docker daemon operations (9 actions)

| Subaction | Scope | Description |
|---|---|---|
| `info` | `synapse:read` | Docker daemon information |
| `df` | `synapse:read` | Docker disk usage |
| `images` | `synapse:read` | List Docker images; `dangling_only` to filter untagged |
| `networks` | `synapse:read` | List Docker networks |
| `volumes` | `synapse:read` | List Docker volumes |
| `pull` | `synapse:write` | Pull a Docker image; requires `host`, `image` |
| `build` | `synapse:write` | Build a Docker image; requires `host`, `context`, `tag`; optional `dockerfile`, `no_cache` |
| `rmi` | `synapse:write` | Remove a Docker image; requires `host`, `image`, `force=true` |
| `prune` | `synapse:write` | Remove unused resources; requires `host`, `prune_target`, `force=true` |

#### `flux container` — Container lifecycle + inspection (14 actions)

| Subaction | Scope | Description |
|---|---|---|
| `list` | `synapse:read` | List containers; optional `state`, `name_filter`, `image_filter`, `label_filter` |
| `inspect` | `synapse:read` | Detailed container info; requires `container_id`; optional `summary` |
| `logs` | `synapse:read` | Container logs; requires `container_id`; optional `lines`, `since`, `until`, `grep`, `stream` |
| `stats` | `synapse:read` | Resource usage stats; optional `container_id` |
| `top` | `synapse:read` | Show running processes; requires `container_id` |
| `search` | `synapse:read` | Full-text search by name/image/labels; requires `query` |
| `start` | `synapse:write` | Start a stopped container; requires `host`, `container_id` |
| `stop` | `synapse:write` | Stop a running container (destructive); requires `host`, `container_id` |
| `restart` | `synapse:write` | Restart a container; requires `host`, `container_id` |
| `pause` | `synapse:write` | Pause a running container; requires `host`, `container_id` |
| `resume` | `synapse:write` | Resume a paused container; requires `host`, `container_id` |
| `pull` | `synapse:write` | Pull latest image for a container; requires `host`, `container_id` |
| `recreate` | `synapse:write` | Recreate container with image pull (destructive); requires `host`, `container_id`; optional `pull` (default true) |
| `exec` | `synapse:write` | Execute command inside container (destructive, execvp); requires `host`, `container_id`, `command` array |

#### `flux host` — Host inspection (9 actions)

| Subaction | Scope | Description |
|---|---|---|
| `status` | `synapse:read` | Check Docker connectivity on a host |
| `info` | `synapse:read` | OS, kernel, architecture |
| `uptime` | `synapse:read` | System uptime |
| `resources` | `synapse:read` | CPU, memory, disk usage |
| `services` | `synapse:read` | Systemd service status; requires `host`; optional `state`, `service` |
| `network` | `synapse:read` | Network interfaces |
| `mounts` | `synapse:read` | Mounted filesystems; requires `host` |
| `ports` | `synapse:read` | Port mappings; requires `host`; optional `protocol`, `limit`, `offset` |
| `doctor` | `synapse:read` | Diagnostic checks; requires `host`; optional `checks` (comma-separated) |

#### `flux compose` — Docker Compose project management (10 actions)

| Subaction | Scope | Description |
|---|---|---|
| `list` | `synapse:read` | List all Docker Compose projects; requires `host` |
| `status` | `synapse:read` | Get project service status; requires `host`, `project`; optional `service` |
| `up` | `synapse:write` | Start a compose project; requires `host`, `project` |
| `down` | `synapse:write` | Stop a compose project (destructive); requires `host`, `project`; optional `remove_volumes`, `force` |
| `restart` | `synapse:write` | Restart a compose project (destructive); requires `host`, `project` |
| `recreate` | `synapse:write` | Recreate compose containers (destructive); requires `host`, `project` |
| `logs` | `synapse:read` | Get project logs; requires `host`, `project`; optional `service`, `lines`, `since` |
| `build` | `synapse:write` | Build compose project images; requires `host`, `project`; optional `service` |
| `pull` | `synapse:write` | Pull compose project images; requires `host`, `project`; optional `service` |
| `refresh` | `synapse:read` | Refresh compose project cache; requires `host` |

#### `flux help` — Auto-generated flux docs

| Action | Scope | Description |
|---|---|---|
| `help` | public | Return flux action reference; optional `topic` (e.g. `"container:list"`), `format` (`markdown`\|`json`) |

---

### `scout` — SSH/local host inspection

#### Scout simple actions (9 actions)

| Action | Scope | Description |
|---|---|---|
| `nodes` | `synapse:read` | List all configured SSH hosts |
| `peek` | `synapse:read` | Read a file or directory listing; requires `host`, `path`; optional `tree`, `depth` |
| `find` | `synapse:read` | Find files by glob; requires `host`, `path`, `pattern`; optional `depth`, `limit` |
| `ps` | `synapse:read` | List processes; requires `host`; optional `sort` (`cpu`\|`mem`\|`pid`\|`time`), `grep`, `user`, `limit` |
| `df` | `synapse:read` | Disk usage; requires `host`; optional `path` |
| `delta` | `synapse:read` | Compare files or content; requires `source_host`, `source_path`; then either `target_host`+`target_path` or `content` |
| `exec` | `synapse:write` | Execute allowlisted command (destructive, execvp); requires `host`, `command`; optional `path`, `args`, `timeout_secs` |
| `emit` | `synapse:write` | Multi-host execution (destructive); requires `targets` array, `command`; optional `args`, `timeout_secs` |
| `beam` | `synapse:write` | File transfer between hosts (destructive); requires `source_host`, `source_path`, `dest_host`, `dest_path` |

#### `scout zfs` — ZFS introspection (3 subactions)

| Subaction | Scope | Description |
|---|---|---|
| `pools` | `synapse:read` | List ZFS pools; requires `host`; optional `pool` name filter |
| `datasets` | `synapse:read` | List ZFS datasets; requires `host`; optional `pool`, `dataset_type`, `recursive` |
| `snapshots` | `synapse:read` | List ZFS snapshots; requires `host`; optional `pool`, `dataset`, `limit` |

#### `scout logs` — Log retrieval (4 subactions)

| Subaction | Scope | Description |
|---|---|---|
| `syslog` | `synapse:read` | Read `/var/log/syslog` (falls back to `/var/log/messages`); requires `host`; optional `lines`, `grep` |
| `journal` | `synapse:read` | Read systemd journal; requires `host`; optional `lines`, `unit`, `priority`, `since`, `until`, `grep` |
| `dmesg` | `synapse:read` | Read kernel ring buffer; requires `host`; optional `lines`, `grep` |
| `auth` | `synapse:read` | Read `/var/log/auth.log` (falls back to `/var/log/secure`); requires `host`; optional `lines`, `grep` |

#### `scout help` — Auto-generated scout docs

| Action | Scope | Description |
|---|---|---|
| `help` | public | Return scout action reference; optional `topic` (e.g. `"zfs:pools"`), `format` (`markdown`\|`json`) |

## Known Parity Gaps

`synapse2` achieves **action-level parity** with `synapse-mcp` — all 59
production actions from `synapse-mcp/docs/INVENTORY.md` are implemented. However,
the following features from the original TypeScript server are **not yet ported**:

### Not ported

| Feature | Description |
|---|---|
| `claude/channel` notifications | Original forwards Docker events and log tails as `notifications/claude/channel` MCP notifications. No equivalent exists in Rust. |
| Templated MCP resources | Original exposes `synapse://hosts/{host}`, `synapse://hosts/{host}/stacks`, `synapse://stacks`, `synapse://stacks/{host}/{stack}`, `synapse://stacks/{host}/{stack}/env`, `synapse://containers/{host}`, `synapse://containers/{host}/{id}`. Rust exposes tool-specific schema and help resources plus read-scoped `synapse://hosts`, `synapse://compose/projects`, `synapse://status`, and `synapse://activity`. |
| Root SSH login gate | Original gates `sshUser=root` through elicitation unless `SYNAPSE_ALLOW_ROOT_LOGIN=true`. Rust has destructive-operation elicitation but no root-login gate. |
| TOFU fingerprint store | Original persists fingerprints to `~/.config/synapse/known_hosts.json` and rejects changed fingerprints. Rust uses strict OpenSSH `known_hosts` with wildcard warnings — different operator behavior. |
| `SYNAPSE_EXCLUDE_HOSTS` | Original env var to exclude hosts from fleet discovery is absent in Rust. |
| `SYNAPSE_MCP_ALLOW_YOLO` | Original "skip all confirmation gates" mode. Rust has `SYNAPSE_MCP_ALLOW_DESTRUCTIVE` (per-restart override, loopback-only), which is not identical. |
| `SYNAPSE_DEBUG_ERRORS` | Original opt-in mode that returns full internal error detail. Rust always sanitizes internal tool errors. |
| `git` in exec allowlist | Original includes `git` in `ALLOWED_READ_COMMANDS` with flag guards. Rust deliberately excludes `git`. |

## Configuration

```bash
SYNAPSE_MCP_HOST=127.0.0.1
SYNAPSE_MCP_PORT=40080
SYNAPSE_MCP_TOKEN=change-me
```

Key environment variables:

| Variable | Default | Description |
|---|---:|---|
| `SYNAPSE_MCP_HOST` | `127.0.0.1` | Bind host for HTTP transport. |
| `SYNAPSE_MCP_PORT` | `40080` | Bind port for HTTP transport. |
| `SYNAPSE_MCP_TOKEN` | unset | Static bearer token for auth. |
| `SYNAPSE_MCP_NO_AUTH` | `false` | Disable auth for loopback development only. |
| `SYNAPSE_NOAUTH` | `false` | Delegate auth/authz to an isolated trusted upstream gateway. |
| `SYNAPSE_MCP_ALLOW_DESTRUCTIVE` | `false` | Skip destructive-operation confirmation prompts (loopback only). |
| `SYNAPSE_MCP_MAX_CONCURRENCY` | `50` | Maximum simultaneous in-flight requests on `/mcp` and `/v1/synapse2`. Excess requests receive HTTP 429 with `Retry-After`. Set to `0` to disable. `/health`, `/ready`, and `/status` are exempt. |

See `.env.example` for the full list of variables and `docs/CONFIG.md` for auth
configuration details.

## Run

```bash
# Start MCP server (stdio transport)
cargo run -- mcp

# Start HTTP server
cargo run -- serve

# CLI examples
cargo run -- flux docker info
cargo run -- flux container list
cargo run -- flux compose list --host myhost
cargo run -- scout nodes
cargo run -- scout exec --host myhost --command hostname
cargo run -- scout zfs pools --host myhost
cargo run -- scout logs journal --host myhost --unit docker
```

MCP examples:

```json
{"name":"flux","arguments":{"action":"docker","subaction":"info"}}
{"name":"flux","arguments":{"action":"container","subaction":"list","state":"running"}}
{"name":"flux","arguments":{"action":"compose","subaction":"status","host":"myhost","project":"mystack"}}
{"name":"scout","arguments":{"action":"nodes"}}
{"name":"scout","arguments":{"action":"exec","host":"myhost","command":"hostname"}}
{"name":"scout","arguments":{"action":"zfs","subaction":"pools","host":"myhost"}}
{"name":"scout","arguments":{"action":"logs","subaction":"journal","host":"myhost","unit":"docker"}}
```

## Architecture

```text
FluxService   (src/flux_service/)  Docker/container/compose/host ops
ScoutService  (src/scout_service/) SSH/exec/fs/zfs/logs ops
      ↓ via SynapseService facade (src/app.rs)
MCP shims     (src/mcp/tools.rs)  tool args → service → Value
CLI shim      (src/cli.rs)        argv → service → stdout
REST layer    (src/api.rs)        POST /v1/synapse2 → service → JSON
```

## Development

```bash
cargo fmt --check
cargo test
cargo clippy -- -D warnings
cargo build --release

just dev     # serve with no auth (loopback, dev mode)
just test    # cargo test
just lint    # clippy
just fmt     # cargo fmt
```

Useful docs:

- `docs/API.md` for full tool contracts
- `docs/CONFIG.md` for environment and auth
- `docs/QUICKSTART.md` for local smoke tests
- `plugins/synapse2/skills/synapse2/SKILL.md` for agent usage guidance
- `tests/parity.rs` for automated parity verification against synapse-mcp INVENTORY

## Documentation

This README is curated for first-run orientation and high-level action discovery.
Source-of-truth docs and code are split as follows:

- `docs/API.md` for the curated action contract reference.
- `docs/MCP_SCHEMA.md` for MCP schema shape and drift expectations.
- `docs/CONFIG.md` for environment and auth policy.
- `docs/QUICKSTART.md` for local smoke tests.
- `docs/PLUGINS.md` and `plugins/synapse2/skills/synapse2/SKILL.md` for agent
  and marketplace usage.
- `docs/generated/openapi.json` for generated OpenAPI output.
- `src/flux_service/`, `src/scout_service/`, and `src/mcp/` for runtime source
  of truth.

## Related Servers

- `unifi-rmcp / rustifi` - UniFi controller REST API bridge.
- `tailscale-rmcp / rustscale` - Tailscale API bridge for devices, users, and tailnet operations.
- `unraid-rmcp / unrust` - Unraid GraphQL bridge for NAS and server management.
- `apprise-rmcp` - Apprise notification fan-out bridge for many delivery backends.
- `gotify-rmcp` - Gotify push notification bridge for sends, messages, apps, and clients.
- `arcane-rmcp` - Arcane Docker management bridge for containers and related resources.
- `yarr-rmcp` - Media-stack bridge for Sonarr, Radarr, Prowlarr, Plex, and related services.
- `ytdl-mcp` - Media download and metadata workflow server.
- `cortex` - Syslog and homelab log aggregation MCP server.
- `axon` - RAG, crawl, scrape, extract, and semantic search project.
- `lab` - Homelab control plane and Labby gateway project.
- `lumen` - Local semantic code search MCP server.
- `nugs` - Project/package management helper for local agent workflows.
- `agentcast` - Agent transcript and activity publishing project.
- `soma` - RMCP scaffold/runtime template for new provider-backed servers.

## License

[MIT](LICENSE)
