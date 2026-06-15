# Phase 1: Code Quality & Architecture Review

Target: `synapse2` @ `a2294fe` (whole repo). Tooling: `scripts/check-rust-module-size.sh` (pass + advisory), `cargo clippy --locked -- -D warnings` (clean, 0 warnings).

## Code Quality Findings

### Critical
- **C-1 — Three independent SSH pools, no connection sharing** (`src/flux_service.rs:79`, `src/scout_service.rs:54`, `src/docker_client/cache.rs:32`). `SynapseService::new()` builds flux + scout independently, each with its own `SshPool` (plus a third inside `DockerClientCache`). A request touching both flux and scout on one host opens 2–3× the SSH sessions needed. DI hooks (`with_pool`, `with_docker_clients`) already exist — wire one shared `Arc<SshPool>` in the constructor.

### High
- **H-1 — `force=true` validation duplicated CLI vs MCP dispatch** (`src/cli/flux.rs:81-82,95-97` vs `src/actions/flux.rs:260-264,272-275`). Same business guard for `docker rmi`/`prune` enforced twice with diverging messages — violates thin-shim. Remove from CLI; let dispatch `ValidationError` propagate.
- **H-3 — `actions/flux.rs` over 400-line soft cap** (432 code lines). Mixes arg structs, `from_flux_args` parser, and four `dispatch_flux_*` functions. Extract dispatchers to sibling `actions/flux_dispatch.rs`.
- **H-4 — `expect()` in production paths** (`src/flux_service/container_driver.rs:62`, `src/fanout.rs:36`). Both rely on implicit invariants not enforced by types. `container_driver` one is a latent panic if `flatten_list_outcome` return shape changes — convert to `if let Some(obj)`. `fanout` one is structurally safe (local semaphore) — at least make the invariant comment explicit.

### Medium
- **M-1 — Fanout error type erasure** (`container_driver.rs:30-33`, `docker_driver.rs:30-33`, ~14 sites). `.map_err(|e| e.to_string())` everywhere drops structured `bollard` error context. Change `FanoutOutcome<T,E>` to carry `anyhow::Error`; stringify only at the serialization boundary.
- **M-2 — Dead section comments in `mcp/tools.rs`** (`:102-105`). "arg helpers" / "help text" separators divide nothing post-refactor. Delete.
- **M-3 — `services_on_host` uses `let _ = state;` suppression** (`flux_service/host.rs:217-225`). Replace with `Option`-based `state.filter(...).map(...)`.
- **M-4 — `api.rs` inlines scout default params** (`:126-135`). `unwrap_or("local")`/`unwrap_or("/tmp")` are business defaults duplicated in the shim; MCP path doesn't apply them, so REST vs MCP `scout.peek` diverge when `path` absent. Push defaults into `from_scout_args`.
- **M-5 — `action_timeout_label` allocates per call** (`actions/dispatch.rs:134-153`). Static arms `.to_owned()` needlessly; return `Cow<'static, str>`.
- **M-6 — `ScoutEmit` clones all target strings** (`actions/dispatch.rs:105-115`). Avoidable temp `Vec<(String,Option<String>)>`; change `resolve_emit_targets` to take slices of refs.
- **M-7 — 17 `// TEMPLATE:` comments in production source** (`cli.rs`, `config.rs`, `server.rs`, `logging.rs`, `token_limit.rs`). Convert guidance to doc comments; delete pure scaffolding.

### Low
- **L-1** test helper duplicates `build_confirmer` logic (`mcp/tools.rs:55-74`).
- **L-2** `main.rs` `auth_config_sources` 55 lines of repetitive pushes; 3-location change pattern for new OAuth fields.
- **L-3** `logging/formatter.rs` 52% non-code comments (cosmetic).
- **L-4** `apps/web/lib/template.ts` hardcodes action specs (drift risk; add CI ID-match check).
- **L-5** `unreachable!` in `flux_service/docker.rs` `prune_single` — caller invariant not type-enforced.

### Convention adherence
Thin-shim mostly honored (breaches: H-1, M-4). No `mod.rs`. No file over hard cap. One soft-cap breach (H-3). Facade thin. No `unwrap!`/`panic!` in prod. Clippy clean. Test-sidecar pattern consistent.

## Architecture Findings

### Critical
- **A-C1 — Domain services import the MCP protocol layer** (`src/flux_service.rs:32`, `src/scout_service.rs:25`: `use crate::mcp::help as help_module`). Inverts dependency direction (domain → protocol). Help text is domain-neutral (CLI/REST/MCP all use it). Move `src/mcp/help.rs` (+`help_topics.rs`) to `src/help.rs`; have `mcp::` import from it. Currently tests of `FluxService::help` transitively load MCP infra.

### High
- **A-H1 — REST surface covers ~14 of 59 actions** (`src/api.rs:91-142`). All scout subactions except nodes/peek/exec absent; container only `list`; host/compose entirely absent. May be intentional (web UI overlay) but `openapi.json` advertises the full shape while only the subset works — contract trap. Either document REST as a limited overlay (annotate `rest_help()`, trim generated OpenAPI) or extend to parity. (Overlaps quality H-2.)
- **A-H2 — REST does not map `ConfirmationDenied`** (`src/api.rs:62-88`). Falls to catch-all 500 instead of 403. MCP maps it correctly via `is_confirmation_denied`. Add a 403 arm.
- **A-H3 — `HostProtocol::Http`/`Https` are dead but valid in config** (`src/synapse.rs:15-16`). Dispatch sites (`flux_service/host.rs:72`, `docker_client/cache.rs:74`) only check `Local`, so `"protocol":"http"` configs silently route as SSH. Implement or reject at load time.

### Medium
- **A-M1 — `scaffold_intent` half-wired** (`src/app.rs:94-98`, `scaffold.rs`). Public facade method not reachable via MCP/CLI/REST, yet `is_validation_error` already downcasts `ScaffoldIntentValidationError`. Wire it through or remove.
- **A-M2 — Flat JSON schema, no discriminated subaction typing** (`src/mcp/schemas.rs:27-153`). All params for all subactions are top-level peers; clients can't pre-validate or generate per-action forms. Consider `oneOf` discriminated on action/subaction.
- **A-M3 — REST dotted-string dispatch fragile/unversioned** (`src/api.rs:102-125`). `trim_start_matches("flux.docker.")` mishandles `flux.docker.foo.bar`. Use `split_once('.')`; consider sharing the `{action,subaction}` envelope.
- **A-M4 — `std::sync::Mutex<Instant>` in async SSH pool** (`src/ssh/pool.rs:27,152`). Safe today (no await under guard) but a fragile precedent. Use `AtomicU64` or `tokio::sync::Mutex`.
- **A-M5 — No rate limiting on `/v1/synapse2` or `/mcp`** (`src/server/routes.rs:102-104`). Only body-limit + CORS. Fanout capped at 8/op and 5-min deadline exist, but no global concurrency backstop — matters under `TrustedGatewayUnscoped`/`SYNAPSE_NOAUTH`. Add `tower` concurrency/rate limit layer.

### Low
- **A-L1** `scout_read_roots` empty for SSH-discovered hosts (`host_config.rs:285`) — verify `validate_scout_read_path` is restrictive, not permissive. **(flag for Phase 2 security)**
- **A-L2** help map can drift from `ACTION_SPECS` — add a test asserting coverage.
- **A-L3** TEMPLATE residue (overlaps M-7).
- **A-L4** hand-written CLI parser: no `--flag=value`, no short flags; consider `clap`/`lexopt`.

### Positive (preserve)
Thin 99-line facade with DI builders; consistent Confirmer gate (4 surfaces, 10s elicit timeout); subaction-level scope dispatch with read-only static bearer default; well-built bounded fanout (correct semaphore placement, ordered, partial-success preserving); segregated `DockerClient` sub-traits with blanket impl; `is_transport_dead` socket-death classification for cache eviction; working module-size enforcement.

## Critical Issues for Phase 2 Context

Security-relevant items to drive the security audit:
- **A-L1 / `scout_read_roots` empty on SSH-discovered hosts** — confirm `validate_scout_read_path` denies by default; if permissive, this is arbitrary remote read (potential Critical).
- **A-H2 — `ConfirmationDenied` → 500** masks the destructive-op gate over REST; verify the gate still *blocks* (only the status code is wrong) and that `SYNAPSE_MCP_ALLOW_DESTRUCTIVE` cannot be reached on non-loopback.
- **A-H3 — dead `Http`/`Https` protocol** silently routes as SSH — misconfiguration → credential/connection surprises.
- **A-M5 — no rate limiting**; combined with fanout + SSH/Docker spawn, assess DoS exposure under `SYNAPSE_NOAUTH`/gateway-unscoped.
- **scout `exec` allowlist + execvp semantics**, **`container exec` argv**, **SSH known_hosts/TOFU handling**, **root-login gating**, **auth/scope enforcement (LoopbackDev/TrustedGatewayUnscoped/bearer/oauth)**, **secret handling in logs/errors**, **CORS/Host allowlists** — all need direct Phase 2 examination.

Performance-relevant items:
- **C-1 SSH pool fragmentation** (connection multiplier).
- **M-1 fanout error stringification** (observability under partial failure).
- **M-5/M-6 per-call allocations** in dispatch hot path.
- Docker cache behavior, fanout 8-cap, runtime budget deadlines, token-limit byte caps — assess under load.
