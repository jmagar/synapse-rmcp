# Phase 4: Best Practices & Standards

Target: `synapse2` @ `a2294fe`. Rust edition/MSRV, deps, web (React 19/Next 16), build, CI/CD, Docker, ops.

## Framework & Language Findings

### Critical
- **B-C1 — Cargo edition declared `2021`, MSRV `1.90`, but code/docs are edition-2024** (`Cargo.toml:20-21`). Compiles today but resolver/`cargo fix --edition`/lint semantics are for the wrong edition. Set `edition = "2024"`, `rust-version = "1.85"`, run `cargo fix --edition`. *(Verify: the agent reported this; confirm `Cargo.toml` actually says 2021 before acting — if it already says 2024, downgrade to non-issue.)*
- **B-C2 — `async_trait` macro on 5 trait families** (`elicitation_gate.rs:125`, `ssh.rs:93`, `flux_service/host.rs:42,50,63`, `docker_client/traits.rs:46,127,153,168,183`). AFIT stable since 1.75 (MSRV 1.90); macro adds a heap alloc/box per call. Migrate to native AFIT or `trait-variant` for the `dyn`+`Send` cases; then drop `async-trait`.

### High
- **B-H1 — Fanout `E = String` erases ~30 error chains** via `.map_err(|e| e.to_string())` (`docker_driver.rs`, `container_driver.rs`, `host_driver.rs`). Change `FanoutOutcome<T,E>` to `anyhow::Error`. *(= quality M-1 / perf P-M1)*
- **B-H2 — `std::sync::Mutex<Instant>` in async SSH pool** (`ssh/pool.rs:27,32-44`). Use `AtomicU64`. *(= arch A-M4 / perf P-M5)*
- **B-H3 — `action_timeout_label` allocates per dispatch** (`actions/dispatch.rs:134-154`) → `Cow<'static,str>`. *(= quality M-5)*
- **B-H4 — Duplicate crate versions: `reqwest` 0.12 + 0.13** (synapse2 vs lab-auth), plus 55 total dup entries (`getrandom` ×3, `hashbrown` ×3, crypto churn from jsonwebtoken/rsa). Bump synapse2 `reqwest` to 0.13 to collapse the most impactful one; rest need lab-auth upgrade.
- **B-H5 — Dockerfile runtime installs full `docker.io` daemon** (`config/Dockerfile:95`) — ~100-200MB for zero benefit (bollard uses the socket). Remove it. *(= CI H-1)*
- **B-H6 — No `[profile.release]` tuning** (`Cargo.toml`). Add `lto="thin"`, `codegen-units=1`, `strip="symbols"` for ~10-15% speed / 20-30% size.

### Medium
- **B-M1** `Cargo.toml:22` description still "Template for building MCP servers with the rmcp crate."
- **B-M2** 43 `// TEMPLATE:` comments in production `src/` (one literally `// # TEMPLATE: Injection attack synapse2`). *(= quality M-7 / docs D-M1)*
- **B-M3** `ValidationError`/`ConfirmationDenied`/scaffold errors hand-roll `Display`+`Error`; add `thiserror = "2"`.
- **B-M4** `async-trait` dep removable after B-C2.
- **B-M5** No `#[non_exhaustive]` on public enums (`SynapseAction`, `HostProtocol`, `AuthPolicy`, `AuthMode`, `FanoutOutcome`).
- **B-M6** `apps/web/biome.json` schema pinned 2.4.16 but biome is 2.5.0.
- **B-M7** No `rust-toolchain.toml`; Docker builder (1.96) diverges from local mise toolchain. Add one.
- **B-M8** Pure UI components (`card/button/badge/separator/input.tsx`) needlessly `"use client"`.
- **B-M9** 18 `React.forwardRef` usages — deprecated in React 19; use native `ref` prop.
- **B-M10** `import * as React` namespace pattern — prefer named imports in React 19.

### Low
- **B-L1** `useCallback` on non-prop fns in `app/page.tsx` (unnecessary).
- **B-L2** tsconfig `target: "ES2017"` → `ES2022`.
- **B-L3** tsconfig missing `noUncheckedIndexedAccess`, `exactOptionalPropertyTypes`, `noImplicitOverride`.
- **B-L4** HEALTHCHECK uses `curl` (~1.8MB) — could use a binary-native `synapse health`.
- **B-L5** `ssh2-config` → `git2` → `openssl-sys` forces native SSL build dep (no clean fix yet; awareness).
- **B-L6** `futures` + `futures-util` both direct deps (redundant).
- **B-L7** `deny.toml` `multiple-versions = "warn"` lets dups accumulate silently.
- **B-L8** Web polling `useEffect` lacks `AbortController` cleanup.

## CI/CD & DevOps Findings

### Critical
- **O-C1 — `docker-publish.yml` + `release.yml` use mutable action tags** (`@v6`/`@v7` etc.) while `ci.yml`/`msrv.yml`/`codeql.yml` pin to SHA. The image-publishing + release-binary workflows (highest-value, `packages:write`+`contents:write`) are exactly the unpinned ones — action-maintainer compromise → token exfil / binary tampering. Pin all to SHA (Dependabot github-actions already configured).
- **O-C2 — `release.yml` `lfs-commit` pushes directly to `main`** via `GITHUB_TOKEN` (lines 130-177), bypassing branch protection, committing release binaries to `bin/` unreviewed. Combined with O-C1 → arbitrary `main` force-push risk. Push to a `binaries` branch + auto-PR, or pull from Release assets instead.

### High
- **O-H1 — Runtime image installs `docker.io`** (`Dockerfile:95`). *(= B-H5)* Remove.
- **O-H2 — No `concurrency:` cancellation in publish/release workflows**; rapid `main` pushes race the `:latest` tag and the Trivy scan target. Add concurrency blocks.
- **O-H3 — Dockerfile `FROM` stages not digest-pinned** (`:24,39,83`); the Dockerfile's own TEMPLATE comment describes the fix. Pin all three.
- **O-H4 — Trivy scans `:latest` registry tag, not the build artifact** (`docker-publish.yml:73-100`) → scans possibly-wrong image under the tag race. Use `docker save` + scan the tarball.
- **O-H5 — No coverage gate** in CI (nextest runs, no llvm-cov/threshold).
- **O-H6 — `install.sh` downloads binary without checksum verification** (block commented out, `:130-138`) though `release.yml` generates `SHA256SUMS`. Activate it.

### Medium
- **O-M1** Dependabot auto-merge relies on branch-protection required checks being set; major `rmcp` bumps allowed.
- **O-M2 — Prod compose binds `0.0.0.0:40080` by default** (`docker-compose.prod.yml:40` + port publish). With S-H1 (`SYNAPSE_NOAUTH`) + mounted docker.sock = root-equiv exposure if firewall/network gaps. Default to `127.0.0.1:...`; document reverse-proxy pattern.
- **O-M3** Runtime base unpinned + Trivy `--ignore-unfixed` → false assurance.
- **O-M4** `release.yml` has no top-level `permissions:` block (inherits broad default).
- **O-M5** No structured JSON logging mode despite `tracing-subscriber` `json` feature present. Add `LOG_FORMAT=json`.
- **O-M6** `entrypoint.sh` only `set -e` (no `-u`/`pipefail`); `SOCK_GID` detection can silently empty → starts without docker.sock access.
- **O-M7** Local root `config.toml` has wrong port (40060 = rarcane, not 40080) and `host=0.0.0.0` (gitignored but misleads `cargo run`).
- **O-M8** No `/metrics`/Prometheus endpoint (only `/health`+`/status`).

### Low
- **O-L1** MSRV job compiles but doesn't run tests.
- **O-L2** `docker-publish.yml` missing top-level `permissions: contents: read`.
- **O-L3** Dependabot allows major `rmcp` bumps automatically.
- **O-L4** RUSTSEC-2023-0071 suppression has no re-eval date/automation. *(ties to S-C1)*
- **O-L5** `install.sh` fragile `grep|sed` JSON parsing.
- **O-L6** No SBOM/provenance attestation on the published image (`provenance:true`/`sbom:true` are one-line adds).

## Cross-cutting
Strong base CI (`ci.yml`/`msrv.yml`/`codeql.yml` SHA-pinned, concurrency, least-priv permissions, nextest, the python drift checks). The gaps cluster in the **publish/release pipeline** (supply-chain pinning, direct-main push, scan-the-artifact) and **template residue** (edition, description, TEMPLATE comments, wrong local config) — consistent with the rmcp-template origin seen across all phases.
