# Phase 2: Security & Performance Review

Target: `synapse2` @ `a2294fe`. Strict + security-focus + performance-critical flags on. Phase 1 handoffs were explicitly verified.

## Security Findings

### Critical
- **S-C1 — RUSTSEC-2023-0071 RSA Marvin timing side-channel in JWT signing** (CWE-208; `deny.toml:18-24`, transitive `lab-auth → jsonwebtoken → rsa 0.9.x`). Advisory is suppressed as "short-lived tokens," but RS256 private key (`/data/auth-jwt.pem`) is long-lived and reused for all OAuth issuance; refresh tokens last 30d (`refresh_token_ttl_secs=2592000`). Key recovery → forge write-scope tokens → destructive fleet ops. Only exploitable in OAuth mode with low-latency network access. Mitigate: track jsonwebtoken constant-time fix; interim rotate key + shorten TTLs (access 900s, refresh 7d); prefer ES256 if lab-auth supports.

### High
- **S-H1 — `TrustedGatewayUnscoped` has no proof of gateway** (CWE-306; `src/server.rs:107`). `SYNAPSE_NOAUTH=true` + non-loopback bind → fully unauthenticated access to `container exec`, `scout exec`, `scout beam`, lifecycle. Any peer/co-located container reaching :40080 gets total fleet control. Mitigate: isolated Docker network as single ingress (document + startup warning); optional shared-secret header or `peer_addr()` IP allowlist (`SYNAPSE_TRUSTED_GATEWAY_IP`).
- **S-H2 — SSH ControlMaster sockets in world-accessible `/tmp`** (CWE-732; `src/ssh/pool.rs:56`). Forwarded Docker sockets are chmod 0600, but the ControlMaster mux socket dir is `/tmp` (predictable name). Local attacker on multi-tenant host connects through it → arbitrary remote commands as SSH user, bypassing allowlist. Mitigate: `control_directory` under `/run/user/<uid>/` or a 0700 process-private dir.
- **S-H3 — `journalctl` filter args unvalidated → argument injection** (CWE-88; `src/scout_service/logs.rs:78-131`). `unit`/`priority`/`since`/`until` passed as argv with no validation. No shell injection (execvp), but `unit="-M container"` smuggles the `--machine` flag → read another container's journal. Mitigate: validate each (reject leading `-`, nulls, length; priority allowlist; time-format guard).
- **S-H4 — REST destructive denial returns HTTP 500 not 403** (CWE-710; `src/api.rs:75-87`). Operation IS blocked (security posture correct) but `ConfirmationDenied` hits the catch-all 500 + logs `error!` — alert fatigue, misleading monitoring. MCP maps it correctly (`rmcp_server.rs:152-158`). Add a 403 arm via `is_confirmation_denied`. *(overlaps arch A-H2)*
- **S-H5 — No rate limiting on `/mcp` or `/v1/synapse2`** (CWE-770; `src/server/routes.rs:48-103`). Only 5-min op deadline + 8-permit per-host SSH semaphore. Sustained `scout emit` fanout exhausts SSH/CPU/FDs. `register_rpm`/`authorize_rpm` only cover OAuth flows. Add `tower_governor`/concurrency layer. *(overlaps arch A-M5)*

### Medium
- **S-M1 — Symlink TOCTOU on remote SSH paths** (CWE-367; `src/scout_service/fs.rs:6-8`, `src/synapse.rs:109-123`). `validate_safe_path` uses *local* `symlink_metadata`; remote path is NotFound locally → passes, then read remotely. Remote symlink in an allowed root → read `/etc/shadow` via `head`. Mitigate: remote `stat -c %F` symlink check before reading.
- **S-M2 — `find -name <pattern>` only checks leading dash** (CWE-88; `src/scout_service/fs.rs:188-207`). Add null-byte + length guards.
- **S-M3 — Empty `scout_read_roots` deny-by-default but undocumented footgun** (CWE-276; `host_config.rs:272-286`, `synapse.rs:133-144`). **VERIFIED SAFE** (denies all). But unhelpful error drives operators to set `["/"]`. Mitigate: actionable error message; warn against `/`; consider default `["/tmp"]` for SSH hosts.
- **S-M4 — `scp_arg` builds `user@host:path`; `ssh_user`/`host` unvalidated** (CWE-77; `src/scout_service/exec.rs:362-371`). Low real injection risk (no shell), but attacker-controlled host config could misparse. Validate `ssh_user` charset; pass port as separate `-p` arg.
- **S-M5 — known_hosts wildcard only warned, not enforced** (CWE-295; `src/ssh/known_hosts.rs:20-36`, `main.rs:73-74`). `KnownHosts::Strict` still accepts a `*` entry → MITM possible. Mitigate: bail on wildcards in non-loopback server mode.
- **S-M6 — `HostProtocol::Http`/`Https` dead → silent SSH fallback** (CWE-1289; `synapse.rs:12-17`, `flux_service/host.rs:71-73`). `"protocol":"https"` attempts SSH with ssh-config creds. Reject at load or implement. *(overlaps arch A-H3)*
- **S-M7 — `/status` & `/openapi.json` unauthenticated** (CWE-200; `routes.rs:84-90`, `api.rs:201-218`). Full schema/action enumeration aids targeting. Mitigate: gate openapi behind bearer on non-loopback; config flag for status.

### Low
- **S-L1** `validate_safe_path` charset over-restrictive (rejects spaces/`@`/`:`) → friction → operators widen roots (`synapse.rs:102-107`).
- **S-L2** `dmesg` error hint suggests `sysctl kernel.dmesg_restrict=0` — weakens host hardening (`logs.rs:204-208`).
- **S-L3** ControlMaster `/tmp` symlink pre-creation attack (overlaps S-H2; `pool.rs:56`).
- **S-L4** `scout exec` args array not length-limited → resource/log spam (`actions/scout.rs:186`).
- **S-L5** `AuthConfig` derives `Debug` incl. `google_client_secret` — leaks if ever `{:?}`-logged (`config.rs:31`). Add redacting `Debug`.

### Verified safe (raised but not vulnerable)
- `scout_read_roots` empty = **deny-by-default** (not allow-all).
- REST `ConfirmationDenied` **blocks** the op (only status code wrong → S-H4).
- `SYNAPSE_MCP_ALLOW_DESTRUCTIVE` + non-loopback = **startup refused** (`main.rs:164-184`).
- **No shell injection** anywhere — all exec/SSH/scp use discrete argv; no `sh -c` in production.
- Scope sentinel `__deny__` and unknown subactions **fail closed**.
- `cargo deny check advisories` clean except S-C1.

## Performance Findings

### Critical
- **P-C1 — SSH pool fragmentation (3 pools/process)** (`flux_service.rs:79,85`, `scout_service.rs:54`, `docker_client/cache.rs:32`). Request touching flux+scout on one host = up to **3N** ControlMaster connections. `with_pool` exists but unused in prod path. Wire one shared `Arc<SshPool>` in `SynapseService::new` → collapses 3N→N (mux multiplexes channels). *(= quality C-1)*
- **P-C2 — Fanout semaphore acquired after `op()` launch** (`src/fanout.rs:209-215`). `op(host_clone)` runs for all N before any permit; cap only throttles `.await` resumption, not start. Pre-`await` work (alloc, arg-build, spawn) bursts all N to the runtime → memory spikes at 20+ hosts. Move `op(host_clone).await` *inside* the permit-guarded block. (Does not reintroduce the DashMap-guard-across-await deadlock the comment guards against.)

### High
- **P-H1 — `load_hosts()` synchronous disk read + SSH-config parse on every fanout request** (`host_config.rs:138,195`, `flux_service.rs:108`). `std::fs::read_to_string` + `SshConfig::parse` (incl. `Include` files) blocks a Tokio worker per `host:None` call. Cache parsed list (`ArcSwap`/`RwLock<Arc<...>>`, TTL/SIGHUP invalidation) or `spawn_blocking`.
- **P-H2 — `find_host_op` probes hosts sequentially** (`container_driver.rs:186-198`). `container_inspect/logs/top` worst case N×RTT when container is on last host. Race with `FuturesUnordered`, first `Ok` wins → ~1×max(RTT).
- **P-H3 — `container_stats` per-container sequential** (`container_driver.rs:95-102`). 50 containers = 50 serial bollard stat streams per host. Use `buffer_unordered(10)`.
- **P-H4 — `target_docker_hosts` runs a full daemon-ID dedup fanout before every op** (`flux_service.rs:118-134`). Every `host:None` docker/container call = N extra `docker info` round-trips, then the real fanout (2 sequential fan-outs). Cache daemon IDs with ~30s TTL.

### Medium
- **P-M1** `.map_err(|e| e.to_string())` at 16+ fanout sites; `FanoutOutcome<T,String>` forces stringification + `error_summary` re-allocs (*= quality M-1*).
- **P-M2** `action_timeout_label` allocates String per dispatch → `Cow<'static,str>` (*= quality M-5*).
- **P-M3** `ScoutEmit` clones all host/path strings per call → borrow slices (*= quality M-6*).
- **P-M4** `should_cap_text_field` linear scan + `keys.cloned().collect()` per JSON object in response capping (`runtime_budget.rs:95-97,150-152`). Use `HashSet`/`match`; iterate `iter_mut`.
- **P-M5** `std::sync::Mutex<Instant>` for `last_used` (`ssh/pool.rs:27`) → `AtomicU64` (*= arch A-M4*).
- **P-M6** Container/scout logs buffered fully before grep (`container_read.rs:318-329`, `logs.rs:294-300`). Apply grep inline during stream collection.
- **P-M7** `dmesg` transfers full ~512KB ring buffer over SSH then tails locally (`logs.rs:141-197`). Tail/level-filter remotely.
- **P-M8** `HostConfig` (multiple `Vec<String>`) cloned whole per fanout task (`fanout.rs:207`). Use `Arc<HostConfig>`.
- **P-M9** `SshPool::checkout` check-then-connect race → up to 8 duplicate 5s connects on cache miss (`ssh/pool.rs:140-175`). Use per-key `tokio::sync::OnceCell` (already used in `docker_client/cache.rs:27`).

### Low
- **P-L1** `ListFilters` cloned per fanout task → `Arc`.
- **P-L2** `search_matches` `to_ascii_lowercase()` per field per container per call (`container_read.rs:188-205`) → `eq_ignore_ascii_case`.
- **P-L3** `error_summary` intermediate Vec then join (`fanout.rs:148-150`).
- **P-L4** `log_output_lines` per-frame `Vec<String>` allocs (`container_read.rs:332-338`).
- **P-L5** `summary_to_value` clones `Option<String>`/label `HashMap` per container → `into_iter()` to move.
- **P-L6** `std::fs::set_permissions` blocking inside async `secure_socket` (`ssh/forward.rs:140`) → `tokio::fs`.

### Scalability
- In-process pools/caches don't share across replicas → M replicas × N hosts duplication; ControlMaster is inherently per-process (run one replica per node — current homelab arch). 
- Fanout cap hard-coded `n.min(8)` (`fanout.rs:200`) → 20 hosts = 3 serial waves; make tunable/configurable.
- No circuit breaker: flaky host eats 5s connect timeouts every fanout.

### Frontend (apps/web) — light pass
Next.js 16 static export, minimal deps (Radix/shadcn/React 19/Tailwind), 10s polling, 20-item activity cap. No material perf issues. Minor: `[item,...prev].slice(0,20)` copies array each update (irrelevant at 20 items).

## Critical Issues for Phase 3 Context

Testing must cover:
- **Security gate regression tests**: destructive-confirmation gate over each surface (S-H4); `AuthPolicy` resolution for all four modes incl. non-loopback refusal of destructive override (verified safe — pin with tests); scope enforcement per subaction incl. `__deny__` / unknown-action fail-closed.
- **Injection guards**: journalctl filter validation (S-H3), find pattern (S-M2), scp/ssh_user (S-M4), remote symlink rejection (S-M1) — add negative tests as fixes land.
- **Path confinement**: `validate_scout_read_path` deny-on-empty-roots (S-M3) and `validate_safe_path` traversal/charset (S-L1).
- **known_hosts wildcard / protocol-variant rejection** (S-M5, S-M6) once enforced.

Documentation must cover:
- REST surface ceiling (~14 of 59 actions) — currently undocumented vs full `openapi.json` (arch A-H1).
- `SYNAPSE_NOAUTH` / `TrustedGatewayUnscoped` deployment safety contract (S-H1).
- `scout_read_roots` requirement for SSH-discovered hosts (S-M3).
- The synapse-mcp parity gaps (archived gap review: claude/channel notifications, MCP resource parity, root-login gate, TOFU store, `SYNAPSE_EXCLUDE_HOSTS`, `SYNAPSE_MCP_ALLOW_YOLO`, `SYNAPSE_DEBUG_ERRORS`).
