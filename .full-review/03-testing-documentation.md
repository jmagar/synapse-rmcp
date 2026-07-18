# Phase 3: Testing & Documentation Review

## Test Coverage Findings

### Critical

None.

### High

1. **Trusted-gateway tests codify the unsafe bypass** (`src/server_tests.rs:30-46`) instead of requiring enforceable peer identity. Replace them with startup rejection and route-level unauthorized-peer tests.
2. **Command-security tests validate executable names, not dangerous arguments** (`src/synapse_tests.rs`, `src/scout_service/exec_tests.rs`). Add table-driven denial tests for `rg --pre`, config/preprocessor options, filesystem operands outside roots, and both `exec`/`emit` paths.
3. **Remote-containment tests cover final symlinks only** (`src/scout_service/fs_tests.rs:255-416`). Add intermediate-symlink and TOCTOU fixtures proving no read command runs outside canonical roots.
4. **Resource-budget tests verify only post-hoc truncation** (`src/runtime_budget_tests.rs`, `src/scout_service/fs_tests.rs`). Add large child/fake-SSH output, large-file delta, and large-tree tests that prove collection is bounded and producers terminate.
5. **Installer download/extraction has no security tests** (`packages/synapse-rmcp/test/` covers platform only). Test downgrade redirects, checksum failures, oversized/partial downloads, traversal/symlink archives, cleanup, and unchanged destinations.
6. **Browser authentication/protected actions have no UI/E2E coverage.** Add mounted bearer/OAuth fixtures and tests for disabled unauthenticated controls, distinct 401/403 handling, session lifecycle, destructive authorization, and confirmation.

### Medium

7. **CLI tests omit malformed inputs exposing parser defects.** Add unknown/duplicate flag, invalid number, argv preservation, and enforced execution-timeout matrices.
8. **Web/backend contract tests compare IDs only, and CI omits `pnpm test`.** Compare normalized required fields/types/enums/destructive metadata and run tests in CI.
9. **Cache identity and runtime lifecycle are not tested through topology changes.** Retarget an alias and assert new connections, eviction task ownership, cache clearing, and awaited shutdown.
10. **Recreate mocks cannot simulate stop failure.** Script per-action failures and prove unexpected stop errors abort before remove/create/start.
11. **HTTP concurrency tests do not exercise overload/queue bounds.** Hold permits, overflow the configured queue, require prompt rejection, health responsiveness, cancellation cleanup, and hard active-work limits.

### Low

12. **Cache tests assert FIFO and use wall-clock sleeps.** Use a controllable clock; test true recency, strict concurrent capacity, and zero capacity.
13. **Coverage and live integration remain optional.** Add coverage thresholds for critical modules, hermetic JSON-RPC/mcporter smoke, and a disposable SSH test service rather than silent skipping.

## Documentation Findings

Generated schema and OpenAPI checks both pass; remaining findings concern semantic/operator documentation.

### Critical

None.

### High

1. **Bearer-auth docs imply writes are available, but static bearer tokens are read-only** (`docs/AUTH.md:11-49`, `README.md:251-255`, `src/server.rs:177-184`). State this explicitly and document OAuth/external authorization requirements for HTTP writes.
2. **The web UI is documented as an operational same-origin REST client without a supported browser-auth contract** (`docs/WEB.md:111-135`, `docs/AUTH.md:16-20`, `apps/web/lib/api.ts`). Document/implement token or OAuth acquisition, storage, refresh, sign-in/out, and authorization headers—or clearly disable protected actions.
3. **Trusted-gateway instructions omit that Synapse enforces no peer boundary** (`docs/AUTH.md:120-131`, `docs/CONFIG.md:35-42`, `docs/DOCKER.md:106-110`). Add a prominent threat boundary, safe isolation example, and negative exposure warning after the implementation is secured.

### Medium

4. **CHANGELOG overstates remote symlink protection** (`CHANGELOG.md:30`). Describe the exact canonical guarantee after remediation; until then acknowledge final-component-only protection.
5. **Docker deployment docs show an all-interface port mapping unlike production Compose** (`docs/DOCKER.md:56-82`, `docker-compose.prod.yml:47-55`). Document `SYNAPSE_MCP_BIND_HOST` and distinguish container bind from host publish address.
6. **`docs/CONFIG.md` omits concurrency and OAuth TTL/rate/static-token settings** relative to `src/config.rs` and `docs/ENV.md`. Generate or mechanically synchronize configuration documentation.
7. **Deployment docs describe nonexistent container auto-bind behavior** (`docs/DEPLOYMENT.md:61-81`). Document the real 127.0.0.1 default and explicit Compose override.

### Low

8. **Runtime image inventory falsely claims Docker CLI is installed** (`docs/DOCKER.md:35-43`, `config/Dockerfile:94-109`). Document Bollard over the mounted socket and actual packages.

## Phase Counts

- Testing: 0 Critical, 6 High, 5 Medium, 2 Low.
- Documentation: 0 Critical, 3 High, 4 Medium, 1 Low.
