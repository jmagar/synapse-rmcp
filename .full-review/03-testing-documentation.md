# Phase 3: Testing & Documentation Review

Target: `synapse2` @ `a2294fe`. Suite: 648+ tests pass, 0 failures (52 sidecar files / 581 fns; 12 integration files / 67 fns; ~90% unit / 10% integration / no e2e). `check-openapi.py` + `check-schema-docs.py` both clean. apps/web tests present but unrunnable (node_modules absent).

## Test Coverage Findings

### Critical
- **T-C1 — No regression test for REST `ConfirmationDenied` → wrong status** (`src/api.rs:63-88`). REST destructive denial falls to the 500 arm; `tests/api_routes.rs` covers 400/403(scope)/413 but never the destructive-denial status. Add test asserting `flux.container.stop` over REST returns 403 (after the fix) — currently would catch the 500 bug. *(ties to S-H4)*

### High
- **T-H1 — Flux driver modules have ZERO tests (1,081 LOC)** (`flux_service/{compose_driver(211),container_driver(339),docker_driver(166),host_driver(365)}.rs`). The thick orchestration layer (fanout across hosts, filter/result shaping, error handling) is untested; only pure helpers are. Largest coverage gap. Add `*_driver_tests.rs` with `MockDockerClient`/mock `SshExecutor` behavior tests (happy + partial-failure + error paths).
- **T-H2 — `scp_arg` ssh_user injection untested** (`scout_service/exec.rs:362-380`). `beam` tests cover path traversal/decline but not malicious `ssh_user` (e.g. `-oProxyCommand=...`) — a real ProxyCommand RCE vector via user-controlled host config. Add a negative test (will fail until S-M4 validation lands).
- **T-H3 — `is_transport_dead` evict-then-retry not wired NOR tested** (`docker_client/cache.rs`, drivers). `is_transport_dead` classification is unit-tested, but `rg is_transport_dead src/flux_service/` returns nothing — drivers never call `invalidate` on BrokenPipe. So stale SSH-forwarded Docker tunnels persist until restart. **This is a real defect, not just a test gap.** Wire invalidate-on-transport-death in drivers + add cycle test.
- **T-H4 — Parity test skips silently without sibling repo** (`tests/parity.rs:55-72`). Well-written (non-vacuity guard, negatives) but returns Ok when `../synapse-mcp/docs/INVENTORY.md` absent; no CI gate ensures the sibling is present, so the parity guarantee may never actually run in CI. Add an embedded minimal action list as fallback.

### Medium
- **T-M1 — journalctl filter values untested for flag injection** (`scout_service/logs.rs:82-130`). `journal_with_unit_filter` asserts happy path only; no negative test for `unit="-u sshd ..."`. Add rejection tests (fail until S-H3 validation lands).
- **T-M2 — Remote symlink TOCTOU untested** (`synapse.rs:109-126`). Symlink rejection tested for local paths only; remote SSH paths skip the check (NotFound branch). Add a mock-`SshExecutor` test where `stat` reports a symlink → must reject (ties to S-M1).
- **T-M3 — Fanout ordering stability under concurrent partial failure untested** (`fanout_tests.rs`). Cap/partial/timeout well covered, but partial-failure ordering test uses sequential mock; add a shuffled-latency test asserting `ok_results()` preserves host order.
- **T-M4 — SSH pool concurrent-checkout race untested** (`ssh/pool.rs:131`). Only sequential reuse + permit count tested; no 8-concurrent-task → 1-session test (ties to P-M9 double-connect).
- **T-M5 — `DenyConfirm` not directly tested** (`elicitation_gate.rs`). REST's hard-block confirmer lacks a direct unit test (1-liner).

### Low
- **T-L1** apps/web tests unrunnable here (no node_modules); `template.test.ts` is a strict golden vs `openapi.json` — make `pnpm install && pnpm test` a mandatory CI gate on template/schema changes.
- **T-L2** `proc_tests.rs` only 2 tests, both negative — no positive `ps`/`df` parse test.
- **T-L3** 10 of 13 doc-tests are `ignored` (token_limit, logging examples not exercised).

### Test quality (positive)
Excellent mock discipline (`SshExecutor`/`DockerClient` traits, no real network); consistent sidecar placement; `elicitation_gate_tests.rs` exemplary (atomic sentinel proves IO gated before confirm); measurement-based fanout concurrency assertion; graceful skip when sshd absent; strong `validate_safe_path`/`validate_scout_read_path` pos+neg coverage; all 4 `AuthPolicyKind` variants covered in `server_tests.rs`; `enforce_destructive_policy` covered inline (`main.rs:328-341`).

## Documentation Findings

### Critical
- **D-C1 — README scope table has 9 wrong annotations (read↔write)** (`README.md` L32,47,49-52,76,81,82 vs `src/actions.rs` scope fns). `flux docker pull`, container `start/restart/pause/resume/pull`, `compose up/build/pull` are documented `synapse:read` but enforced `synapse:write`. Operators issuing read-only tokens per the README will hit runtime denials. Code is correct; docs are wrong.
- **D-C2 — API.md claims false full-surface parity** (`docs/API.md:4`: "All three surfaces … produce identical results"). REST actually exposes 13 of 59 actions; the other 46 return `UnknownAction`. The accurate caveat exists 17 lines later but says "some" (drastically understates 46/59). Rewrite the opening to state MCP/CLI are primary (full 59) and REST is a 13-action shim. *(ties to A-H1)*

### High
- **D-H1 — `SYNAPSE_API_URL`/`SYNAPSE_API_KEY` documented but never read** (`docs/ENV.md:26-27`, `.env.example:23-24`; only appear in `config_tests.rs`). Template residue from rmcp-template's upstream-API wrapper. Remove from docs + example.
- **D-H2 — CLAUDE.md env table omits ~11 OAuth sub-vars + Docker/logging vars** that `config.rs:284-316` parses and `docs/ENV.md` documents (TTLs, RPM limits, key/sqlite paths, redirect URIs, disable-static-token; plus `DOCKER_GID`, `DOCKER_NETWORK`, `SYNAPSE2_VERSION`, `SYNAPSE_MCP_HOST_PORT`, `NO_COLOR`, `FORCE_COLOR`). Add them.
- **D-H3 — Unacknowledged synapse-mcp parity gaps** (`README.md` "all 59 production actions"; `CHANGELOG` "full parity (B17)"). Not ported: `claude/channel` notifications, templated MCP resources (`synapse://hosts/{host}` etc.), root-SSH-login gate, `SYNAPSE_EXCLUDE_HOSTS`, `SYNAPSE_MCP_ALLOW_YOLO`, `SYNAPSE_DEBUG_ERRORS`, TOFU fingerprint store. Add a "Known Parity Gaps" section / `docs/PARITY.md`; qualify the claim to "action-level parity."
- **D-H4 — ARCHITECTURE.md & CLAUDE.md module maps miss ~20 src modules** (incl. cross-cutting `fanout.rs`, `elicitation_gate.rs`, `cache.rs`, `formatters.rs`, `logging.rs`, `scaffold.rs`, `synapse.rs`, and `mcp/{help,resources,response,prompts}.rs`). Add one-line descriptions.

### Medium
- **D-M1 — `// TEMPLATE:` markers remain in production source** (`token_limit.rs`, `logging.rs`+`logging/{aurora,formatter}.rs`, `server.rs:120`, `cli.rs`, `config.rs:208`). Pollutes `cargo doc`; one even reads `// # TEMPLATE: Injection attack synapse2`. Convert valuable ones to design-note rustdoc; delete the rest. *(= quality M-7)*
- **D-M2 — CHANGELOG [0.1.0] enumerates template artifacts** (`example` tool, `EXAMPLE_MCP_TOKEN`, `~/.example`) without context — reads as if synapse2 began as "example." Add a scaffold note or collapse 0.x history.
- **D-M3 — MCP_SCHEMA.md scope column misleads** (all flux families shown `synapse:read`; subaction resolution requires write for mutating ones). Add a footnote.
- **D-M4 — ENV.md `.env` block uses fictional `synapse2.com` for the unused vars** (`docs/ENV.md:95-96`). Remove with D-H1.
- **D-M5 — ARCHITECTURE.md `AppState` snippet missing `auth_state` field** (`server.rs`). Sync snippet.

### Low
- **D-L1** CHANGELOG top `<!-- TEMPLATE: -->` reminders visible publicly.
- **D-L2** `src/lib.rs` test helpers use `synapse2.synapse2.com` / `admin@synapse2.com` placeholders → use `example.com`.
- **D-L3** API.md parity-verification output stated without noting the sibling-repo skip.
- **D-L4** Missing module `//!` rustdoc on `scaffold.rs`, `color_policy.rs`; audit `compose.rs`, `synapse.rs`.

## Cross-cutting note for final report
**T-H3 surfaces a genuine functional defect** (transport-death eviction documented + classified but never invoked by drivers) — promote it in the final report beyond "test gap." Strong inline rustdoc exists on the hard modules (`ssh.rs`, `fanout.rs`, `elicitation_gate.rs`, `docker_client.rs`); the doc problems are accuracy drift (README scopes, API.md parity, env vars, parity gaps, module maps), not absence.
