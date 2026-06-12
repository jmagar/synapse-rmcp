---
title: "Architecture"
doc_type: "guide"
status: "active"
owner: "synapse2"
audience:
  - "contributors"
  - "agents"
scope: "synapse2"
source_of_truth: false
upstream_refs:
  - "src/actions.rs"
  - "docs/API.md"
last_reviewed: "2026-06-12"
---

# Architecture

Synapse2 is a Rust MCP + CLI server for local infrastructure workflows. It
exposes two MCP tools:

- `flux` for Docker, container, compose, and host status actions.
- `scout` for SSH-backed node discovery, filesystem inspection, process/log/ZFS
  introspection, and gated exec/emit/beam actions.

The architecture is intentionally layered so transports stay thin and business
logic stays testable.

## Layer diagram

```
SynapseService (src/app.rs)          → thin facade over domain services
FluxService    (src/flux_service.rs) → Docker/container/compose/host logic
ScoutService   (src/scout_service.rs) → SSH/filesystem/process/log/ZFS logic
MCP shims      (src/mcp/tools.rs) → parse JSON args → call service → return Value
CLI shim       (src/cli.rs)       → parse argv → call service → print
REST shim      (src/api.rs)       → parse HTTP JSON → call service → return JSON
```

**The golden rule:** if you are writing business logic in `mcp/tools.rs`,
`cli.rs`, `api.rs`, or `main.rs`, move it to the service layer.

## Module layout

```
src/
  app.rs            ← SynapseService facade; no domain accumulation
  flux_service.rs   ← FluxService domain entry point
  flux_service/     ← focused Docker/container/compose/host modules
  scout_service.rs  ← ScoutService domain entry point
  scout_service/    ← focused SSH/filesystem/process/log/ZFS modules
  host_config.rs    ← shared host topology repository
  config.rs         ← Config structs + env overrides
  api.rs            ← REST API handlers (api_dispatch, health, status)
  server.rs         ← AppState, AuthPolicy, build_auth_layer
  server/
    routes.rs       ← axum router: wires mcp + api + auth + SPA fallback
  mcp.rs            ← MCP module entry: submodule decls + re-exports only
  mcp/
    tools.rs        ← thin shim: parse args → call service → return Value
    schemas.rs      ← tool JSON schema + ACTIONS const
    rmcp_server.rs  ← ServerHandler impl (tools, resources, prompts, scopes)
    prompts.rs      ← MCP prompt definitions
    transport.rs    ← Streamable HTTP transport wiring and session lifecycle
  cli.rs            ← thin shim: parse args → call service → format/print
  cli/
    doctor.rs       ← pre-flight checks: env, connectivity, config validation
    setup.rs        ← interactive first-run / plugin setup wizard
    watch.rs        ← polls /health and emits state-change lines for plugin monitor
  token_limit.rs    ← token budget enforcement for MCP response payloads
  web.rs            ← optional static web UI: asset serving and SPA fallback
  lib.rs            ← pub modules + test helpers (testing::*)
  main.rs           ← mode dispatch ONLY (serve_mcp / serve_stdio / run_cli)
```

## Core files

| File | Responsibility |
|---|---|
| `src/app.rs` | Thin `SynapseService` facade over the domain services. |
| `src/flux_service.rs` | `FluxService` implementation for Docker, container, compose, and host actions. |
| `src/scout_service.rs` | `ScoutService` implementation for SSH, filesystem, process, log, and ZFS actions. |
| `src/host_config.rs` | Shared host topology loading from `SYNAPSE_HOSTS_CONFIG`, `SYNAPSE_CONFIG_FILE`, and `~/.ssh/config`. |
| `src/actions.rs` | Canonical action metadata, parsing, REST dispatch helpers. |
| `src/mcp/tools.rs` | MCP `flux` and `scout` tool dispatch plus elicitation-gated actions. |
| `src/mcp/schemas.rs` | Tool input schema generated from action metadata. |
| `src/mcp/rmcp_server.rs` | `ServerHandler`, scope enforcement, tools/resources/prompts. |
| `src/server.rs` | Axum server startup, auth policy resolution, app state. |
| `src/server/routes.rs` | HTTP routes for MCP, health, status, REST API, and web assets. |
| `src/config.rs` | Environment/config loading and safe defaults. |
| `src/main.rs` | Mode dispatch: HTTP server, stdio MCP, CLI, setup commands. |

## AppState

```rust
#[derive(Clone)]
pub struct AppState {
    pub config: McpConfig,        // MCP server config (host, port, auth settings)
    pub auth_policy: AuthPolicy,  // LoopbackDev | Mounted
    pub service: SynapseService,  // Thin facade over FluxService + ScoutService
}
```

`AppState` is cloned per-request by the RMCP framework. Keep it cheap to clone:
the service facade and its domain services are cloneable handles over shared
state.

## Route composition

All surfaces (MCP, REST API, web UI) share **one binary on one port**:

```
Port 40080
  ├── /mcp                  → Streamable HTTP MCP transport
  ├── /health               → Unauthenticated liveness probe
  ├── /status               → Public redacted runtime state
  ├── /v1/synapse2          → REST API action dispatch
  ├── /.well-known/*        → OAuth metadata (when auth_mode=oauth)
  └── /*                    → SPA fallback (serves embedded web UI)
```

```rust
// src/server/routes.rs
pub fn router(state: AppState) -> Router {
    let public = Router::new()
        .route("/health", get(health))
        .route("/status", get(status));

    let api = Router::new()
        .route("/v1/synapse2", post(api_dispatch))
        .route_layer(auth_layer.clone());

    let mcp = Router::new()
        .nest_service("/mcp", streamable_http_service(state.clone(), mcp_config));

    Router::new()
        .merge(public)
        .merge(api)
        .merge(mcp)
        .with_state(state)
        .layer(TraceLayer::new_for_http())
}
```

## CLI thin shim pattern

`src/cli.rs` follows the same shim discipline as `mcp/tools.rs`. The canonical shape:

```rust
// cli.rs — binary module
use synapse2::app::SynapseService;

pub enum CliCommand {
    Things,
    Thing { id: String },
    DeleteThing { id: String, confirm: bool },
}

impl CliCommand {
    pub fn parse(args: &[String]) -> Result<(Self, bool)> {
        let json    = args.iter().any(|a| a == "--json");
        let confirm = args.iter().any(|a| a == "--confirm");
        let rest: Vec<&str> = args.iter()
            .filter(|a| a.as_str() != "--json" && a.as_str() != "--confirm")
            .map(String::as_str).collect();

        let cmd = match rest.as_slice() {
            ["things"]         => Self::Things,
            ["thing", id, ..]  => Self::Thing { id: id.to_string() },
            ["delete", id, ..] => Self::DeleteThing { id: id.to_string(), confirm },
            other => bail!("unknown command: {}\n\nRun `synapse --help`", other.join(" ")),
        };
        Ok((cmd, json))
    }
}

pub async fn run(service: &SynapseService, cmd: CliCommand, json: bool) -> Result<()> {
    let (label, data) = match cmd {
        CliCommand::Things                            => ("things", service.list_things().await?),
        CliCommand::Thing { ref id }                  => ("thing",  service.get_thing(id).await?),
        CliCommand::DeleteThing { ref id, confirm }   => ("delete", service.delete_thing(id, confirm).await?),
    };
    if json { println!("{}", serde_json::to_string_pretty(&data)?); }
    else    { print_human(label, &data); }
    Ok(())
}
```

`parse()` extracts flags and dispatches to variants — no defaults, no validation, no domain logic. `run()` calls the service and formats output. That's it.

## What "thin shim" means

`mcp/tools.rs` does exactly three things per action:
1. Extract named arguments from the `Value` args object
2. Call the corresponding `state.service.method()`
3. Return the `Value` result

`cli.rs` does exactly three things per command:
1. Parse CLI flags/positional args into typed values
2. Call the corresponding `service.method()`
3. Format and print the result (or pass `--json` through verbatim)

Zero validation, zero defaults, zero error message crafting in shims. All of that lives in `app.rs`.

## Split rules — when to make a directory vs a file

| Surface | Split into a directory when… |
|---|---|
| `<service>/` | upstream API has ≥ 2 resource groups |
| `app/` | service methods exceed one focused domain |
| `api/handlers/` | ≥ 2 resource groups; each file stays thin (≤ 200 lines) |
| `web/pages/` | ≥ 3 page routes |

## File size targets

| Threshold | Action |
|---|---|
| ≤ 250 non-test lines | Target — ideal module size |
| > 400 non-test lines | Must add split/refactor note in PR |
| > 600 non-test lines | Requires documented exception |
| > 800 total lines | Must split unless generated/fixture/schema |

## Modern Rust requirements

- No `mod.rs` files — use named module files (`mcp.rs` + `mcp/tools.rs`)
- Rust 2021 edition minimum, target 2024 where possible
- `thiserror` for structured error types in the service layer
- `?` operator chains over nested `match`
- Avoid `unwrap()`/`expect()` in production paths

## Invariants

- Shims do not contain business logic.
- All action metadata starts in `src/actions.rs`.
- Read actions require `synapse:read`; write/destructive actions require `synapse:write`; `help` is public.
- Stdio is local trusted transport; HTTP is protected unless in loopback or explicit trusted-gateway mode.
- Plugin setup is binary-owned: hook scripts delegate to `synapse setup plugin-hook`.

See `docs/PATTERNS.md` §1, §7, §A1, §45 for full pattern details.
