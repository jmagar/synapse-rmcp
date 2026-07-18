# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

<!-- TEMPLATE: When releasing, move items from [Unreleased] to a new version section.
               Format: ## [X.Y.Z] — YYYY-MM-DD
               Use Added / Changed / Deprecated / Removed / Fixed / Security headers. -->

## [0.5.3](https://github.com/jmagar/synapse-rmcp/compare/v0.5.2...v0.5.3) (2026-07-11)


### Fixed

* update trivy action pin ([066a2b3](https://github.com/jmagar/synapse-rmcp/commit/066a2b355c2f8022880d4541108536cc4f13f514))

## [Unreleased]

<!-- TEMPLATE: Add changes here as you work. They move to a version section on release. -->

### Security

- **RBAC scope corrections** — `flux docker pull`, `flux container start/restart/pause/resume/pull`, and `flux compose up/build/pull` are now documented (and enforced) as `synapse:write`. Read-only tokens that appeared to allow these actions will be denied. Enforcement was already write-scoped in practice; this corrects the documentation and schema to match.
- **Global concurrency cap** — new `SYNAPSE_MCP_MAX_CONCURRENCY` env var (default `50`) caps simultaneous in-flight requests on `/mcp` and `/v1/synapse2`. Excess requests receive HTTP 429 with `Retry-After`; `/health`, `/ready`, and `/status` are exempt. Set to `0` to disable.
- **`/openapi.json` now requires auth** on non-loopback (`Mounted`) deployments to prevent unauthenticated schema enumeration. The endpoint remains public under `LoopbackDev`.
- **`journalctl` filter args validated** — `unit`, `priority`, `since`, and `until` arguments for `scout logs journal` are now validated before being passed to `journalctl`. Extended priority range syntax (e.g. `err..warning`) is rejected; only the RFC 5424 named levels are accepted.
- **`scout beam` hardened** — `ssh_user` and `host` are validated; the SSH port is now passed as a separate `-P` flag rather than interpolated into the host string.
- **Remote `scout peek`/`find`/`delta` reject symlinks** — a pre-read `stat` check over SSH rejects symbolic links before content is read (S-M1 TOCTOU guard).
- **Secrets redacted in debug output** — `SYNAPSE_MCP_TOKEN` and the Google OAuth client secret are redacted to `[REDACTED]` in all `Debug` formatting and log output.
- **Removed unauthenticated trusted-gateway mode.** Non-loopback HTTP now always requires local bearer or OAuth authentication, including behind a gateway.

### Fixed

- **Agent-native MCP contracts** now provide conditional operation schemas, caller-visible structured execution errors, accurate quick-start calls and CLI naming, tool-specific schema resources, and conservative safety annotations.
- **Shared runtime context** adds read-scoped `synapse://status` and `synapse://activity` resources plus a bounded `/activity` feed consumed by the operator dashboard across REST and MCP calls.
- **REST destructive-op denials return HTTP 403** (was 500) for both flux and scout actions when no elicitation channel is available.
- **Stale SSH-forwarded Docker sockets evicted on transport death** across all docker/container read operations. Previously only some code paths triggered eviction.
- **Scout CLI and execution contracts** now preserve argv, reject unknown or malformed options, and enforce requested command deadlines.
- **Operator web request safety** now keeps bearer credentials in memory, gates actions from authoritative server capabilities, synchronously blocks duplicate submissions, aborts superseded requests, rejects stale completions and polling responses, and renders activity from server-owned sequence IDs.
- **Cache and mutation error handling** now provides strict bounded LRU behavior and aborts log/recreate workflows on unexpected command failures.

### Changed

- **JSON logging mode** — set `LOG_FORMAT=json` or `RUST_LOG_FORMAT=json` to emit structured NDJSON log lines instead of the default human-readable format.
- **Rust edition 2024** — workspace updated to `edition = "2024"`. Release profile now uses `lto = "thin"` and `strip = "symbols"`.
- **`rust-toolchain.toml` added** — pins the toolchain channel for reproducible builds.
- **Dropped arm64 support.** The `Docker Publish` workflow previously also built
  `linux/arm64` under QEMU emulation, which made the emulated Rust release build
  exceed the job timeout and cancel every run; it now builds `linux/amd64` only.
  `install.sh` no longer claims to support arm64 hosts (no `aarch64` release
  binary is published) and instead points arm64 users to a source build, and the
  CI docs were corrected to match. Re-add arm64 via a native runner matrix if it
  is needed.

## [0.5.2] — 2026-06-11

### Fixed

- **Config / `.env` are now honored inside Docker.** `Config::load` searched
  `$HOME/.synapse2/` and the current directory, but the container bind-mounts the
  appdata dir at `/data` (where `default_data_dir()` resolves in-container), so
  the mounted `config.toml` / `.env` were never read. Loading now searches the
  resolved service data dir (`/data` in Docker, `~/.synapse2` bare-metal, or
  `SYNAPSE_HOME`) and the current directory, with appdata `.env` taking priority
  over a current-directory `.env` while existing process env remains final.
- **Appdata `.env` now seeds runtime environment variables before logging.**
  Values such as `RUST_LOG`, `NO_COLOR`, and upstream `SYNAPSE_API_*` settings
  now work from the same appdata `.env` as the typed MCP auth settings.
- **Docker image includes the SSH client required by `scout` and remote `flux`.**
  The runtime image now installs `openssh-client`, matching the `openssh`
  crate's native-mux backend requirement.

### Changed

- **SSH fleet auto-discovery works in Docker.** The image now sets
  `HOME=/home/synapse` and creates `/home/synapse/.ssh`, and
  `docker-compose.prod.yml` bind-mounts the operator's `~/.ssh` (read-only) there,
  so hosts in `~/.ssh/config` are auto-discovered into the fleet (`host_config.rs`).
- **`flux` Docker tools can reach the daemon.** `docker-compose.prod.yml` now
  bind-mounts `/var/run/docker.sock` and requires `DOCKER_GID` so the UID-1000
  service has the host's docker group. `entrypoint.sh` still detects the socket
  GID for direct root-started `docker run` use.
- **Production compose no longer defaults to trusted-gateway no-auth.** The
  explicit `SYNAPSE_NOAUTH=true` compatibility mode remains supported for
  isolated Labby/proxy deployments where the gateway enforces auth/authz.
- **`.env.example` slimmed to secrets, URLs, and runtime vars only;** non-secret
  server tuning is documented in `config.example.toml`, which now also explains
  host-topology discovery from `~/.ssh/config`.
- **The example TOML no longer documents ignored fields.** Unsupported entries
  such as the fake upstream `[synapse2]` block and config-level OAuth allowed
  email list were removed from `config.example.toml`.

## [0.5.1] — 2026-06-06

### Added

- **Reached action-level synapse-mcp parity (B17)** — all 59 production actions from
  `synapse-mcp/docs/INVENTORY.md` are now implemented and verified in synapse2.
  Note: some TypeScript-original features are not yet ported (claude/channel
  notifications, templated MCP resources, root-SSH gate, TOFU fingerprint store,
  `SYNAPSE_EXCLUDE_HOSTS`, `SYNAPSE_MCP_ALLOW_YOLO`, `SYNAPSE_DEBUG_ERRORS`, `git`
  in the exec allowlist). See README "Known Parity Gaps" for details.

  **`flux docker`** (9 actions): `info`, `df`, `images`, `networks`, `volumes`,
  `pull`, `build`, `rmi`, `prune` — full Docker daemon management including
  destructive image/resource operations (B10).

  **`flux container`** (14 actions): `list`, `inspect`, `logs`, `stats`, `top`,
  `search` (read-only, B8) + `start`, `stop`, `restart`, `pause`, `resume`,
  `pull`, `recreate`, `exec` (lifecycle, B9) — full container lifecycle with
  B5 Confirmer gate for destructive ops.

  **`flux host`** (9 actions): `status`, `info`, `uptime`, `resources`,
  `services`, `network`, `mounts`, `ports`, `doctor` — host-level inspection
  including systemd services and port mappings (B11).

  **`flux compose`** (10 actions): `list`, `status`, `up`, `down`, `restart`,
  `recreate`, `logs`, `build`, `pull`, `refresh` — full Compose project
  management with per-project and per-service scope (B12 + B13).

  **`scout` simple actions** (9 actions): `nodes`, `peek`, `find`, `ps`, `df`,
  `delta`, `exec`, `emit`, `beam` — SSH/local host inspection and guarded
  command execution with the exec allowlist and B5 gating (B14).

  **`scout zfs`** (3 subactions): `pools`, `datasets`, `snapshots` — read-only
  ZFS introspection via SSH (B15).

  **`scout logs`** (4 subactions): `syslog`, `journal`, `dmesg`, `auth` —
  remote log retrieval with local grep filtering (B15).

  **`flux help` + `scout help`** — topic-aware per-subaction documentation with
  `topic` and `format` params; 59 help topics in `src/mcp/help.rs` (B16).

  Parity is now automatically verified by `tests/parity.rs`, which parses
  `../synapse-mcp/docs/INVENTORY.md` and asserts every row is covered by
  `ACTION_SPECS` and the help map. Skips gracefully if the sibling repo is
  absent. Run with:
  ```
  cargo test --test parity -- --nocapture
  ```
  Expected output: `synapse-mcp parity: 61 rows parsed → 61 matched, 0 missing`

- **MCP resources expansion + topic-aware help (B16)**:
  - `list_resources` now returns 6 URIs: `synapse://schema/flux`, `synapse://schema/scout`, `synapse://hosts`, `synapse://compose/projects`, `synapse://help/flux`, `synapse://help/scout`.
  - `read_resource` delegates to new `src/mcp/resources.rs` which serves all 6 resources. Schema resources return full tool JSON schemas; hosts/compose return live data from host repo and ComposeDiscovery cache; help resources return full per-domain markdown.
  - `flux(action="help", topic="container:list")` and `scout(action="help", topic="exec")` now return per-subaction markdown documentation. Unknown topics return a clear error. `topic` omitted → topic index (backwards-compatible legacy shape + `topics` key). `format="json"` wraps the response in `{topic, text}`.
  - `src/mcp/help.rs` — static `HashMap<&'static str, &'static str>` with 59 topic entries covering all flux (`docker:*`, `container:*`, `host:*`, `compose:*`) and scout (`nodes`, `peek`, `find`, `ps`, `df`, `delta`, `exec`, `emit`, `beam`, `zfs:*`, `logs:*`) topics.
  - `src/mcp/resources.rs` — resource enumeration (`all_resources()`) and read handlers.
  - `SynapseAction::FluxHelp`/`ScoutHelp` variants updated from unit to struct variants carrying `topic: Option<String>` and `format: Option<String>`.
  - flux and scout inputSchemas updated with `topic` and `format` properties (required for `additionalProperties: false` compliance).
  - CLAUDE.md "How to add an action" checklist updated with step 8: add a help-text entry to `src/mcp/help.rs` keyed by `"<domain>:<action>"`.

- **container lifecycle subactions (B9)** — 8 new `flux container` subactions reachable from both MCP (`flux` tool) and CLI (`synapse2 flux container …`):
  - `start`, `restart`, `pause`, `resume` — simple lifecycle ops; ungated (parity with synapse-mcp).
  - `stop` — DESTRUCTIVE (B5 Confirmer gate before any IO); maps to Docker `stop`.
  - `pull` — pull the latest image for THIS container's image (distinct from `docker pull`); ungated.
  - `recreate` — DESTRUCTIVE (B5 Confirmer gate). Sequence: inspect → (pull if pull=true, default true) → stop → remove → create with same config (volumes/networks preserved from `HostConfig`/`NetworkingConfig`) → start. Returns original/new container IDs, image ref, and pull flag.
  - `exec` — DESTRUCTIVE (B5 Confirmer gate). One-shot 3-step bollard exec: `create_exec` → `start_exec` → `inspect_exec`. Never shells via `sh -c` (pure execvp). Returns combined stdout + stderr + exit code. Timeout clamped to `[1000, 300000]ms`, default 30000ms, wrapped in `tokio::time::timeout`.
- `src/flux_service/container_lifecycle.rs` — pure per-host lifecycle ops (`lifecycle_action_on_host`, `pull_image_on_host`, `recreate_on_host`, `exec_on_host`). Operates on `&dyn ContainerOps`/`&dyn ImageOps` — fully testable with `MockDockerClient`.
- `src/flux_service/container_lifecycle_tests.rs` — 16 unit tests covering verb mapping, recreate action sequence (inspect→stop→remove→create→start), pull ordering, exec empty-command guard, timeout clamp, and `split_image_ref` edge cases.
- `create_container` added to `ContainerOps` trait, `BollardClient` impl, and `MockDockerClient` (with `create_container_response` field for test scripting).
- `optional_u64_param` helper added to `crate::actions` shared param helpers.

- **scout ZFS + logs (B15)** — 7 new read-only scout subactions reachable from both MCP (`scout` tool `action=zfs|logs`) and CLI (`synapse2 scout zfs|logs …`):
  - `zfs pools` — `zpool list [<pool>]` via SSH; returns tabular `{header, rows}`. Clean error when ZFS not installed.
  - `zfs datasets` — `zfs list [-t <type>] [-r] [<pool>]`; type allowlist enforced (`filesystem|volume|snapshot|bookmark|all`).
  - `zfs snapshots` — `zfs list -t snapshot [-r <dataset|pool>]`; optional `limit` truncates rows (with `truncated` flag).
  - `logs syslog` — `tail -n <lines> /var/log/syslog`; falls back to `/var/log/messages` (RHEL/CentOS).
  - `logs journal` — `journalctl -n <lines> --no-pager [-u unit] [-p priority] [--since] [--until]`; all filter args as typed argv (no shell injection).
  - `logs dmesg` — `dmesg --color=never`; permission errors returned as structured help rather than hard-failing.
  - `logs auth` — `tail -n <lines> /var/log/auth.log`; falls back to `/var/log/secure`.
  - Grep filtering applied **locally** after remote retrieval (injection-safe) for all four log subactions.
  - Lines clamped to `[1, 500]`, default 100.
- `src/scout_service/zfs.rs` — `pools`, `datasets`, `snapshots` implementations + tabular parser.
- `src/scout_service/logs.rs` — `syslog`, `journal`, `dmesg`, `auth` implementations + `apply_grep` helper.
- `src/scout_service/zfs_tests.rs`, `logs_tests.rs` — unit tests: tabular parsing, limit truncation, fallback path (syslog→messages, auth.log→secure), dmesg permission error, grep filter, invalid type rejection, ZFS-not-installed error.
- `ScoutZfsArgs`, `ScoutLogsArgs` arg structs added to `actions/scout.rs` with `from_scout_args` arms.
- `SynapseAction::ScoutZfs`, `SynapseAction::ScoutLogs` variants added; dispatch via `dispatch_scout_zfs`/`dispatch_scout_logs` helpers.
- `ACTION_SPECS` updated: `zfs` and `logs` (read-only, `READ_SCOPE`, `destructive: false`).
- `src/mcp/schemas.rs` — `scout` tool action enum expanded to include `zfs` and `logs`; subaction, pool/dataset/type/limit and log filter params documented.
- CLI: `synapse2 scout zfs pools|datasets|snapshots` and `synapse2 scout logs syslog|journal|dmesg|auth` fully wired.

- **scout simple actions (B14)** — 9 scout subactions reachable from both MCP (`scout` tool) and CLI (`synapse2 scout …`):
  - `nodes` — list all configured hosts (previously MVP, now fully wired to `ScoutService`).
  - `peek` — read a file or directory listing on a host. Adds `tree` (bool) and `depth` (1–10) params. Symlink check via `validate_safe_path` + remote via SSH `stat`+`cat`/`ls`.
  - `find` — `find <path> -maxdepth <N> -name <pattern> -type f` on a host. Pattern validated to reject leading `-` (option injection guard).
  - `ps` — `ps aux --sort -<field>` with sort/grep/user/limit filters. Sort field validated against allowlist `[cpu, mem, pid, time]`.
  - `df` — `df -h [path]` disk usage. Path validated via `validate_safe_path`.
  - `delta` — compare a remote file against either another `{host,path}` or inline `content` (≤1 MB). Produces unified diff header with labelled lines.
  - `exec` — run an allowlisted command on a host. **DESTRUCTIVE** (gated via B5 Confirmer). Command validated by `validate_command`+`EXEC_ALLOWLIST` BEFORE any IO. `git` is explicitly NOT in the allowlist (B0 security review). `path` is the optional working directory for local hosts only; SSH exec is execvp-style (no shell, no `cd`). HARD INVARIANT: never wraps commands in `sh -c`.
  - `emit` — run an allowlisted command across multiple `{host, path}` targets with per-host timeout. **DESTRUCTIVE** — single Confirmer gate fires before the multi-host loop. Per-host validation runs individually. Returns `all_ok`/`partial_success`/`all_failed` status.
  - `beam` — transfer a file between two `{host,path}` endpoints via `scp` subprocess (not a shell; args are typed). **DESTRUCTIVE** — gated via B5. Both paths validated by `validate_safe_path`.
- `src/scout_service/fs.rs` — `peek`, `find`, `delta` implementations.
- `src/scout_service/proc.rs` — `ps`, `df` implementations.
- `src/scout_service/exec.rs` — `exec`, `emit`, `beam` implementations with B5 gating.
- `src/scout_service/fs_tests.rs`, `proc_tests.rs`, `exec_tests.rs` — unit tests covering validator rejection, `git`/`rm` denylist, confirmer decline, partial-success fanout.
- `ScoutService` extended with SSH executor (`Arc<dyn SshExecutor>`) + `with_ssh_executor` injector.
- `SynapseAction` extended with `ScoutPeek` (now with `tree`/`depth`), `ScoutFind`, `ScoutPs`, `ScoutDf`, `ScoutDelta`, `ScoutExec`, `ScoutEmit`, `ScoutBeam` variants.
- `ACTION_SPECS` updated: `find`, `ps`, `df`, `delta` (read, `READ_SCOPE`); `exec`, `emit`, `beam` (destructive, `WRITE_SCOPE`).
- `src/mcp/schemas.rs` — `scout` tool schema expanded to all 9 actions with full parameter documentation.

### Security

- B14 security note: `git` removed from exec allowlist (B0 fix: `git -c core.editor=...` RCE vector). Tests assert `git` is rejected.
- `validate_safe_path` enforces absolute paths, no `..`, no unsafe chars, no local symlinks for all peek/find/delta/beam paths. Remote path validation is syntactic-only (symlink check uses local `symlink_metadata` — no-op for paths not on the local fs).
- SSH exec is always execvp-style (`SshExecutor::exec(program, args[])`) — the `sh -c` shell injection invariant is locked and tested.
- `emit` multi-host exec validates the command against the global allowlist before confirmation, then again per-host (host-specific allowlist may differ).

- **flux compose operations (B13)** — 10 compose subactions reachable from both MCP (`flux` tool `action=compose`) and CLI (`synapse2 flux compose …`):
  - `list` — run `docker compose ls --format json` on a host; returns discovered projects. Also invalidates the B12 cache via `refresh`.
  - `refresh` — invalidate the B12 compose discovery cache for a host, forcing a re-scan on the next `list`.
  - `status` — `docker compose ps --format json` for a project; optional `service` filter.
  - `up` — `docker compose up -d`. Not destructive (creates, not destroys).
  - `down` — `docker compose down [--volumes]`. **DESTRUCTIVE** — gated via B5 elicitation (`confirmer.require`). `remove_volumes=true` requires `force=true` (validated at service layer before the gate runs, not in the shim).
  - `restart` — `docker compose restart`. **DESTRUCTIVE** — gated via B5 elicitation.
  - `recreate` — `docker compose up -d --force-recreate`. **DESTRUCTIVE** — gated via B5 elicitation.
  - `logs` — `docker compose logs [--tail N] [--since T] [<service>]`. Duration/timestamp forms passed through to docker compose unchanged. Not gated.
  - `build` — `docker compose build [<service>]`. Not gated (parity with synapse-mcp; does not destroy state).
  - `pull` — `docker compose pull [<service>]`. Not gated.
  - All ops resolve the project's compose file via B12's `ComposeDiscovery.list()`, then invoke `docker compose -f <config_file> <subcommand>` over the B11 `HostExec` seam (local or SSH).
- `src/flux_service/compose_ops.rs` — pure per-host compose op functions (`up_on_host`, `down_on_host`, `restart_on_host`, `recreate_on_host`, `status_on_host`, `logs_on_host`, `build_on_host`, `pull_on_host`, `list_on_host`) + `DownArgs` + `validate_down_args` + `ComposeLogOptions`.
- `src/flux_service/compose_ops_tests.rs` — unit tests: argv construction for all 10 subactions, `validate_down_args` cross-field validation (remove_volumes/force), confirmer accept/deny behaviour.
- **flux host full parity (B11)** — 9 host subactions reachable from both MCP (`flux` tool `action=host`) and CLI (`synapse2 flux host …`):
  - `status` — Docker connectivity probe + container count + failed systemd service count (best-effort), fans out across all hosts when `host` unspecified.
  - `info` — `uname -a` output, fans out when `host` unspecified.
  - `uptime` — `uptime` output, fans out when `host` unspecified.
  - `resources` — CPU (load avg from `/proc/loadavg`), memory (`/proc/meminfo`), disk (`df -h`), fans out when `host` unspecified.
  - `services` — `systemctl list-units --type=service --no-pager` with optional `state` and `service` name filters; single-host.
  - `network` — `ip addr show` (falls back to `cat /proc/net/dev`); fans out when `host` unspecified.
  - `mounts` — `df -h` output; single-host.
  - `ports` — container port mappings via bollard with optional `protocol` filter and `limit`/`offset` pagination; single-host.
  - `doctor` — aggregated health checks: `docker`, `containers` (bollard), `resources`, `network`, `services`, `logs` (journald), `processes`; accepts `checks` list to run a subset; single-host.
  - Local hosts (`HostProtocol::Local` / `localhost`) use `std::process::Command`; remote hosts use the SSH pool (execvp-style, no shell).
  - Shell commands are developer-hardcoded — `validate_command` / `EXEC_ALLOWLIST` guard only applies to user-supplied `scout exec` input.
- `src/flux_service/host.rs` — pure per-host functions + `HostExec` seam (`LocalExec` / `RemoteExec`), `CheckResult`/`CheckStatus` types, `strip_systemctl_footer`, `parse_meminfo`, `parse_loadavg`.
- `src/flux_service/host_tests.rs` — 22 unit tests with a `MockExec` returning canned `CommandOutput`; no live SSH server required.
- `HostArgs` params struct in `actions.rs` (mirrors `ContainerArgs`/`DockerArgs` pattern); `dispatch_flux_host` dispatcher.
- `ssh_pool` field on `FluxService` — shared `Arc<SshPool>` for host shell commands.

- **flux docker full parity (B10)** — `info`, `df`, `images` (with `dangling_only`), `networks`, `volumes`, `pull`, `build`, `rmi`, `prune` (target: containers/images/volumes/networks/buildcache/all), via bollard, reachable from MCP (`flux` tool) and CLI. Read-only ops fan out across hosts; `pull`/`build`/`rmi`/`prune` are single-host. `build`/`rmi`/`prune` are gated through the B5 destructive-op elicitation gate (decline → hard error unless `SYNAPSE_MCP_ALLOW_DESTRUCTIVE=true`). `build` shells out to `docker build` (bollard's build needs a streamed tar); all other ops use bollard. New `src/flux_service/docker.rs` with build-context/Dockerfile validation and `PruneTarget` parsing.

- **flux container read-only ops (B8)** — replaced the local-`docker`-CLI stubs for `list`/`inspect`/`logs` with bollard-backed implementations and added `stats`, `top`, and `search`, all reachable from both MCP (`flux` tool) and CLI (`synapse2 flux container …`):
  - `list` — filters: `state` (running/exited/paused/restarting/all), `name_filter`, `image_filter` (case-insensitive substring), `label_filter` (`key=value`, bollard server-side).
  - `logs` — one-shot tail (`follow=false`); `lines` (1–500, default 50), `since`/`until` (ISO 8601, unix seconds, or relative `"1h"`/`"30m"`), `grep` (substring filter on lines), `stream` (stdout/stderr/both).
  - `inspect` — `summary` flag for abbreviated output.
  - `stats` — one-shot resource stats for one container, or all containers on the host(s) when `container_id` is omitted.
  - `top` — running processes (bollard-wrapped `docker top`).
  - `search` — full-text substring match over container name + image + labels (client-side grep, not a bollard server-side filter).
  - Multi-host behavior: `list`/`search`/`stats(no id)` fan out across all configured hosts and return a flat, host-tagged list with a `partial` flag and per-host `errors`; `inspect`/`logs`/`top` target a named host or fan out to find the owning host (first match wins).
  - `response_format` (`markdown`/`json`) is validated at the shim per the B4 contract; output-rendering wiring remains a separate codebase-wide concern (actions return structured JSON today).
- `src/flux_service/container_read.rs` (+ `_tests.rs`) — pure per-host container ops over `&dyn ContainerOps`, fully unit-testable with `MockDockerClient` (no live daemon). Includes `parse_time_spec` for log time ranges.
- `MockDockerClient` gains scriptable `log_frames` / `stats_frames` fields for B8 streaming tests.
- `ContainerArgs` — shared boxed parameter struct for `flux container` subactions, used by both `SynapseAction::FluxContainer` and the CLI `Command`.

## [0.5.0] — 2026-05-28

### Added

- `src/cache.rs` / `src/cache_tests.rs` — generic synchronous `Cache<K, V>` trait and `MemoryCache` implementation: per-entry TTL (default 60s), bounded capacity with LRU eviction (default 10k entries), lazy expiration, and `DashMap`-backed thread safety. Adds the `dashmap` dependency.
- `allow_destructive` config option (`SYNAPSE_MCP_ALLOW_DESTRUCTIVE` env var, default `false`) gating destructive shell operations. Documented in `config.example.toml`.

### Security

- `validate_safe_path` now requires absolute paths and rejects symlinks via `symlink_metadata` before any read — prevents symlink-based arbitrary file reads in world-writable directories.
- Removed `git` from the exec allowlist (`EXEC_ALLOWLIST`).
- The MCP server returns a generic `invalid request` error to unauthenticated callers for unknown actions and scope mismatches, preventing unauthenticated probes from enumerating valid action names.
- The server refuses to start when `SYNAPSE_MCP_ALLOW_DESTRUCTIVE=true` is set on a non-loopback bind address, and warns when enabled on loopback.
- Documented the CORS allowlist policy in `src/server/routes.rs` and `config.example.toml`: auth (bearer/OAuth) is the primary control; CORS is defense-in-depth for browser clients.

### Changed

- Dependency bumps via Dependabot: `serde_json` 1.0.149 → 1.0.150, `EmbarkStudios/cargo-deny-action`, and (web app) `postcss` 8.5.14 → 8.5.15, `@types/react`.

## [0.4.0] — 2026-05-14

### Added

- `.github/workflows/codeql.yml` — CodeQL SAST analysis on push to main and weekly scheduled scan; results surface in the GitHub Security tab.
- `.github/workflows/cargo-deny.yml` — license compliance, duplicate dependency, advisory, and source checks via `cargo-deny`.
- `.github/workflows/msrv.yml` — compiles against the declared `rust-version` to catch MSRV regressions early.

## [0.3.0] — 2026-05-14

### Added

- `src/cli/watch.rs` — `example watch` subcommand for live file-system monitoring.
- `plugins/example/monitors/` — plugin monitor definitions for event-driven automation.
- `plugins/example/gemini-extension.json` — Gemini extension manifest for multi-platform plugin distribution.
- `.github/dependabot.yml` + `.github/workflows/dependabot-auto-merge.yml` — automated dependency updates with auto-merge for minor/patch bumps.
- `scripts/asciicheck.py`, `scripts/check-blob-size.py`, `scripts/check-dependency-updates.sh`, `scripts/check-file-size.sh`, `scripts/check-runtime-current.sh`, `scripts/validate-plugin-layout.sh`, `scripts/blob-size-allowlist.txt` — repository validation and quality scripts.
- `tests/plugin_contract.rs` — plugin contract integration tests.
- `docs/PLUGINS.md` — documentation for the plugin system and distribution model.
- `plugins/README.md`, `plugins/example/README.md`, `plugins/example/CLAUDE.md` — plugin-level documentation and agent guidance.
- `apps/web/README.md`, `xtask/README.md`, `tests/README.md`, `scripts/README.md` — README coverage for every major directory.
- `.claude/` — Claude Code project settings for agent-assisted development.

### Changed

- `plugins/example/hooks/plugin-setup.sh` — significant simplification; reduced from ~500 to ~50 lines by extracting reusable logic and removing duplication.
- `Justfile` — expanded with additional recipes covering plugin validation, script checks, and workflow shortcuts.
- `lefthook.yml` — pre-commit hook additions aligned with new script suite.
- `AGENTS.md`, `CLAUDE.md` — updated agent and AI tooling guidance to reflect current project structure.
- `README.md`, `docs/PATTERNS.md` — documentation refreshed for new scripts and plugin layout.

## [0.2.0] — 2026-05-14

### Changed

- Split `src/mcp.rs` into three focused modules: `src/server.rs` (`AppState`, `AuthPolicy`, `build_auth_layer`), `src/server/routes.rs` (Axum router wiring), and `src/api.rs` (REST API handlers). `src/mcp/` now contains only MCP protocol concerns (tools, schemas, prompts, server handler).
- `mcp/rmcp_server.rs` and `mcp/tools.rs` now import `AppState`/`AuthPolicy` from `crate::server` instead of `super`.
- `allowed_origins` visibility widened from `pub(super)` to `pub` to support cross-module access from `server/routes.rs`.
- Updated `src/lib.rs` and `src/main.rs` to reflect new module layout (`pub mod api`, `pub mod server`).

### Added

- `deny.toml` — `cargo-deny` configuration enforcing license allowlist, banning `openssl`/`openssl-sys`, denying yanked crates, and restricting dependency sources to crates.io and `github.com/jmagar/lab.git`. RUSTSEC-2023-0071 acknowledged with rationale.
- `apps/web/CLAUDE.md` — guidance for using the Aurora design system shadcn registry in the Next.js web app: install commands, token conventions, full component catalog, and usage rules.
- `.git/hooks/pre-commit` — enforces the no-`mod.rs` rule at commit time; blocks any staged `mod.rs` file with a clear error message.
- `docs/PATTERNS.md` updated: §1/§1a module layouts reflect new `server`/`api` structure with all `mod.rs` references removed; §5 auth section headers updated; §45 No mod.rs section now includes the git hook script; §A1/§A2 advanced patterns updated to match actual file locations.

### Removed

- `src/mcp/routes.rs` — moved to `src/server/routes.rs`.
- Several obsolete scripts: `backup.sh`, `check-runtime-current.sh`, `plugin-setup.sh`, `reset-db.sh`, `smoke-test.sh`, `test-check-runtime-current.sh`, `validate-marketplace.sh`.
- `docs/server-json-guide.md` — content superseded by `docs/MCP-REGISTRY-PUBLISH-GUIDE.md`.

## [0.1.0] — 2026-05-13

### Added

- Layered architecture: `ExampleClient` (transport) → `ExampleService` (business logic) → MCP/CLI shims
- Action-based dispatch: single `example` MCP tool with `action` parameter routing
- Both transports: Streamable HTTP (`example serve`) and stdio (`example mcp`)
- Bearer token authentication via `EXAMPLE_MCP_TOKEN`
- Google OAuth authentication via `EXAMPLE_MCP_AUTH_MODE=oauth` (issues RS256 JWTs)
- Loopback/no-auth mode for local development
- MCP elicitation support (`elicit_name` action, spec 2025-06-18) with graceful fallback
- MCP resources: exposes tool schema at `example://schema/mcp-tool`
- MCP prompts: `quick_start` prompt
- CLI with `greet`, `echo`, and `status` subcommands
- Test helpers: `loopback_state()` and `bearer_state()` for credential-free integration tests
- `AuthPolicy` enum making auth choice explicit at construction time
- CORS, Host header validation, request body size limiting built-in
- `resolve_auth_policy_kind()` — refuses to bind `0.0.0.0` without auth (Pattern §27)
- `default_data_dir()` — detects container vs bare-metal, returns `/data` or `~/.example`
- `entrypoint.sh` — Docker entrypoint with permission setup and privilege drop to UID 1000
- `xtask` crate with `dist`, `ci`, `symlink-docs`, `check-env` commands
- `.config/nextest.toml` — nextest configuration with `default` and `ci` profiles
- `taplo.toml` — TOML formatter configuration
- `lefthook.yml` — minimal pre-commit hooks (diff_check, toml_fmt, env_guard)
- `.github/workflows/ci.yml` — CI: fmt, clippy, nextest, taplo, audit, gitleaks
- `.github/workflows/docker-publish.yml` — multi-platform Docker build + Trivy scan
- `.github/workflows/release.yml` — release binaries for linux/amd64 and linux/arm64
- `config.example.toml` — fully annotated config template
- `.env.example` — documented secrets template
- `CHANGELOG.md` following Keep a Changelog format
- Workspace structure: root crate + `xtask/` member
- `symlink-docs` and `symlink-docs-inline` Justfile recipes
