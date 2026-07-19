# Phase 4: Best Practices & Standards

## Framework & Language Findings

### Critical / High

None.

### Medium

1. **Aurora's documented token source is not loaded** (`apps/web/app/globals.css`, `apps/web/components/aurora.css`, UI components). Twenty-four referenced status/surface variables are absent from the loaded stylesheet. Import one canonical token sheet, remove duplication, and add a variable-definition guard.
2. **Release and npm CI bypass the repository's SHA-pinning standard** (`.github/workflows/ci.yml:230-235`, `.github/workflows/release.yml`). Pin every third-party action to a full reviewed SHA and enforce it with automation.
3. **Published artifacts use unconstrained/different Rust toolchains** (`rust-toolchain.toml`, `.github/workflows/release.yml`, `config/Dockerfile`). Define one production compiler version and keep MSRV as a separate compatibility job.
4. **Local filesystem work blocks Tokio executor threads** (`src/scout_service/fs.rs:43-76,147-151,327-335`). Move bounded local I/O to `tokio::fs` or a single `spawn_blocking` operation.

### Low

5. **Static API reference is unnecessarily a client component** (`apps/web/app/api/page.tsx`). Remove `"use client"`.
6. **Seven Radix packages are unused** (`apps/web/package.json`). Remove them, refresh the lockfile, and add dependency hygiene checking.
7. **Legacy Docker CLI code duplicates Bollard and blocks inside async** (`src/docker.rs`, `src/lib.rs`). Remove it or migrate it to the shared bounded async path.
8. **Health polling permits overlapping/stale requests** (`apps/web/app/page.tsx`, `apps/web/lib/api.ts`). Use abortable completion-driven polling.

## CI/CD & DevOps Findings

### Critical

None.

### High

1. **Repair automation targets `example-mcp` and can stop the wrong workload** (`scripts/repair.sh`, `Justfile`). Use canonical Synapse identifiers, validate the resolved target, and add hermetic tests.
2. **Container scanning occurs after publication, scans the wrong tag for releases, and loses failed SARIF** (`.github/workflows/docker-publish.yml`). Scan the exact pre-push digest, upload SARIF under `if: always()`, and publish/promote only after success.
3. **Production Compose does not reference the published image** (`docker-compose.prod.yml`, workflow/server metadata). Use `ghcr.io/jmagar/synapse`, prefer digests, and add a contract test.

### Medium

4. **Runtime freshness verification uses template identifiers** (`scripts/check-runtime-current.sh`, `Justfile`, docs). Adopt Synapse systemd/container defaults and test both modes.
5. **PRs never build/smoke the deployable container** (`.github/workflows/ci.yml`). Add Docker build, Compose config, and `/health`/`/ready` smoke before merge.
6. **Production Compose disables entrypoint permission repair with fixed UID/GID 1000** (`docker-compose.prod.yml`, `entrypoint.sh`). Restore audited privilege drop or parameterize/preflight identity and test non-1000 hosts.
7. **Release-readiness gate fails on a stale literal assertion** (`scripts/pre-release-check.sh`, release workflow). Validate semantic artifacts and run the gate in CI.
8. **Auth smoke uses obsolete variables and port 3000** (`scripts/test-mcp-auth.sh`, docs). Use `SYNAPSE_MCP_TOKEN`, `SYNAPSE_MCP_URL`, port 40080, safe temp files, and a verbatim CI fixture.
9. **Health monitoring cannot distinguish liveness from readiness** (`src/api.rs`, `src/cli/watch.rs`, Compose, docs). Keep `/health` for liveness, add bounded `/ready`, and point deployment readiness at it.
10. **Persistent log size is capped only at startup** (`src/logging.rs`). Add size-aware rotation/retention with diagnostics.

### Low

11. **Standalone CLI generation remains a broken template workflow** (`scripts/generate-cli.sh`, `Justfile`). Use Synapse identifiers and a valid MCP initialize/tools-list flow with smoke coverage.

## Phase Counts

- Framework/Language: 0 Critical, 0 High, 4 Medium, 4 Low.
- CI/CD/DevOps: 0 Critical, 3 High, 7 Medium, 1 Low.
