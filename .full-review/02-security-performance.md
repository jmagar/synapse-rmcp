# Phase 2: Security & Performance Review

## Security Findings

### Critical

None.

### High

1. **Trusted-gateway mode exposes full fleet control to every reachable peer** (`src/server.rs:89-108,152-177`, `src/main.rs:225-267`, `src/mcp/rmcp_server.rs:310-332`, `src/api.rs:195-202`; CWE-306, estimated CVSS 8.8). `SYNAPSE_NOAUTH=true` on a non-loopback bind mounts no authentication, bypasses scopes, and does not enforce the documented gateway IP. Enforce mTLS, peer CIDRs, or local auth; fail startup without a cryptographic/enforced trust constraint.
2. **The Scout command allowlist is bypassable through `rg --pre`** (`src/synapse.rs:78-88,221-235`, `src/scout_service/exec.rs:61-101,148-205`; CWE-78, estimated CVSS 8.1). Top-level executable validation allows `rg` while forwarding arguments that can spawn a denylisted shell. Implement per-command argument schemas and reject execution-capable options/configuration.
3. **Remote Scout read roots are bypassable through intermediate symlinks** (`src/synapse.rs:129-146,198-218`, `src/scout_service/fs.rs:43-62,89-143,327-358`; CWE-22/CWE-59, estimated CVSS 7.5). Lexical remote-prefix validation plus final-path `stat` permits `/allowed/link -> /etc`. Canonicalize on the remote host immediately before access or use no-follow handle-based reads.
4. **A privileged scheduled workflow installs and executes unpinned npm code** (`.github/workflows/openwiki-update.yml:9-36,75-105`; CWE-829). Pin package version/integrity and action SHAs, disable persisted credentials, and split network generation from minimal PR permissions.

### Medium

5. **Scout read-command arguments bypass configured roots and sensitive-path controls** (`src/synapse.rs:129-235`, `src/scout_service/exec.rs:61-101,148-205`; CWE-200/CWE-862). Typed operand validation must apply canonical read-root and sensitive-file restrictions to every filesystem argument.
6. **Binary installers fail open or omit integrity verification** (`packages/synapse-rmcp/scripts/install.js:22-84`, `packages/synapse-rmcp/lib/platform.js:24-35`, `scripts/install.sh:60-90`, `install.sh:125-151`; CWE-494). Require HTTPS without downgrade, verified checksums/attestations, bounded downloads, and safe archive-entry validation.
7. **OAuth/JWT dependency retains RUSTSEC-2023-0071 (Marvin RSA timing)** (`Cargo.lock`, `Cargo.toml:61-63`, `deny.toml:16-25`; CWE-208, CVSS 5.9). Track/migrate to a constant-time signing implementation or non-RSA algorithm and retain only a documented time-bounded exception.
8. **Current-working-directory `.env` can reconfigure a privileged service** (`src/config/env.rs:39-74`, `src/config.rs:304-340`; CWE-15). Restrict cwd dotenv loading to explicit development mode/file paths and validate ownership/mode.

### Low

9. **Locked Vite 8.0.14 has two Windows dev-server advisories** (`apps/web/package.json`, `apps/web/pnpm-lock.yaml`; CVE-2026-53571 and CVE-2026-53632). Update to Vite 8.0.16+ through a compatible Vitest release/override.
10. **Yanked `spin 0.9.8` fails the dependency policy gate** (`Cargo.lock`, `deny.toml`). Perform a targeted compatible update and keep yanked dependencies denied.
11. **Release metadata is interpolated directly into privileged shell source** (`.github/workflows/release.yml:26-63`; CWE-78). Pass event/input values through `env:` and validate tags strictly.

Security verification: `cargo audit --no-fetch` confirmed RUSTSEC-2023-0071; `cargo deny check` failed only on yanked `spin`; `pnpm audit` found the two Vite advisories; the only gitleaks match was a verified test marker.

## Performance Findings

### Critical

None.

### High

1. **Subprocess and SSH output is fully buffered before response caps apply** (`src/runtime_budget.rs:54-72`, `src/ssh/pool.rs:378-390`, `src/actions/dispatch.rs:31-36`). Output-heavy concurrent calls can exhaust memory. Drain stdout/stderr concurrently into bounded buffers with explicit truncation metadata.
2. **Scout `delta` reads arbitrary files and runs a quadratic diff** (`src/scout_service/fs.rs:270-389`). Cap/stat operands before reading, use `spawn_blocking`, a bounded Myers/patience algorithm, and output ceilings.
3. **HTTP concurrency limiting creates an unbounded waiter queue** (`src/server/routes.rs:87-124,153-174`). Reserve capacity at admission or use bounded Tower load-shed/concurrency middleware, short queue deadlines, and 429/503 responses.
4. **`scout find` and tree previews cap results only after fully capturing `find` output** (`src/scout_service/fs.rs:173-250`). Enforce a maximum at execution time with bounded walking/streaming and terminate once the limit is met.

### Medium

5. **SSH sessions, Docker clients, and forwarded sockets lack runtime lifecycle wiring** (`src/ssh/pool.rs:283-337`, `src/docker_client/cache.rs:53-103`, `src/main.rs`). Start eviction, cap caches, clear clients, and await pool shutdown; overlaps Phase 1 architecture lifecycle finding.
6. **All-container stats collection is serial within each host** (`src/flux_service/container_driver.rs:91-135`). Use bounded unordered concurrency while retaining stable output and per-container failures.
7. **Compose discovery performs serial N+1 remote reads and permits cache stampedes** (`src/compose.rs:277-395`). Add per-key single flight and bounded/batched remote reads.
8. **Hostless single-container operations probe hosts sequentially** (`src/flux_service/container_driver.rs:185-239,315-332`). Resolve ownership with bounded concurrency, first-success cancellation, and shared logic.

### Low

9. **Fanout materializes one future/config clone per host despite active-work bounds** (`src/fanout.rs:195-246`). Use a lazy stream with `buffer_unordered(8)`.
10. **Host topology is synchronously reread/reparsed on async request paths** (`src/host_config.rs:130-215`, `src/flux_service.rs:114-119`). Cache immutable revisioned snapshots and reload through `spawn_blocking` when sources change.

## Critical Issues for Phase 3 Context

- No Critical-severity findings were reported.
- Tests must cover gateway trust enforcement, per-command argument policy (including `rg --pre`), remote symlink containment, installer integrity failures, bounded output/find/delta behavior, and bounded request admission.
- Documentation must state the browser-auth contract, trusted-gateway threat boundary, remote read-root guarantees, timeout behavior, output/result caps, dependency exceptions, and installer provenance.

## Phase Counts

- Security: 0 Critical, 4 High, 4 Medium, 3 Low.
- Performance: 0 Critical, 4 High, 4 Medium, 2 Low.
