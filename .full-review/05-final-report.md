# Comprehensive Code Review Report — synapse2

## Review Target

Full repository review of **synapse2** @ commit `a2294fe` (branch `claude/adoring-hellman-0dc9e5`, even with `main`, clean tree). synapse2 is the Rust MCP + CLI + REST server (binary `synapse`) for local Synapse workflows — a full-parity Rust port of `synapse-mcp` exposing two action-dispatched tools: `flux` (Docker via bollard) and `scout` (SSH/fs/proc/zfs/logs/transfer/allowlisted-exec). ~34.8k LOC Rust + ~2.8k LOC TS/TSX (Next.js 16 static UI). Flags: security-focus, performance-critical, strict-mode.

Phases: 1 Quality & Architecture · 2 Security & Performance · 3 Testing & Documentation · 4 Best Practices & Standards. Tooling run during review: `clippy -D warnings` (clean), `check-rust-module-size.sh` (pass+advisory), `check-openapi.py`/`check-schema-docs.py` (clean), `cargo deny` (clean except 1 suppressed), 648+ tests pass.

## Executive Summary

synapse2 is a **well-engineered, structurally disciplined codebase** — thin facade + driver pattern, segregated Docker traits, a consistently-applied Confirmer gate, fail-closed scope checks, bounded fanout, working module-size enforcement, clean clippy, and a green test suite. The highest-fear security items were **verified safe by design**: no shell injection anywhere (all argv, no `sh -c`), `scout_read_roots` denies by default, the destructive override is refused on non-loopback binds, and scopes fail closed.

The real risk concentrates in three areas: (1) a small number of **genuine correctness/security defects** — transport-death cache eviction that is classified but never invoked, `journalctl` arg-injection, remote symlink TOCTOU, and the `SYNAPSE_NOAUTH` deployment trust model; (2) **release/supply-chain pipeline hardening** — unpinned actions and a direct-push-to-`main` in the publish/release workflows; and (3) pervasive **rmcp-template residue** — wrong Cargo edition/description, 40+ `TEMPLATE:` comments, documented-but-unused env vars, and README scope annotations that contradict the enforced scopes. Nothing here is a production-down emergency, but several items are exploitable or misleading enough to fix before the next release.

## Findings by Priority

### Critical Issues (P0 — Must Fix Immediately)

- **[Security S-C1] RUSTSEC-2023-0071 RSA Marvin timing side-channel** in OAuth JWT signing (`deny.toml`, transitive `lab-auth→jsonwebtoken→rsa`). Long-lived RS256 key + 30-day refresh tokens. **Only exploitable in OAuth mode with network proximity** — P0 *only if* OAuth mode is deployed; otherwise P2. Mitigate: shorten TTLs + rotate key now; track jsonwebtoken constant-time fix; prefer ES256.
- **[Testing T-H3 → functional defect] Transport-death eviction never invoked.** `is_transport_dead` is classified and unit-tested but `rg is_transport_dead src/flux_service/` returns nothing — drivers never call `invalidate` on a broken SSH-forwarded Docker socket, so **stale tunnels persist until process restart**, causing sticky failures. Wire invalidate-on-transport-death in the drivers + add a cycle test. (Promoted from "test gap" — it's a real bug.)
- **[CI O-C1] Publish/release workflows use mutable action tags** while base CI is SHA-pinned — the `packages:write`+`contents:write` workflows are the unpinned ones. Pin to SHA.
- **[CI O-C2] `release.yml` pushes directly to `main`** via `GITHUB_TOKEN`, bypassing branch protection and committing unreviewed binaries. Route through a `binaries` branch / Release assets.

### High Priority (P1 — Fix Before Next Release)

Security:
- **S-H1** `SYNAPSE_NOAUTH=true` on a non-loopback bind = **fully unauthenticated fleet control** with no gateway proof (`server.rs:107`). Most serious exploitable-as-shipped risk. Document the isolation contract; add optional IP allowlist / shared-secret. *(compounds with CI O-M2: prod compose binds `0.0.0.0` by default + mounts docker.sock.)*
- **S-H2** SSH ControlMaster sockets in world-accessible `/tmp` → local session hijack bypassing allowlist (`ssh/pool.rs:56`). Use `/run/user/<uid>/` or 0700 dir.
- **S-H3** `journalctl` filter args unvalidated → flag injection (`-M`/`--machine` reads other containers' journals) (`scout_service/logs.rs`). Validate unit/priority/since/until.
- **S-H4 / A-H2** REST `ConfirmationDenied` → 500 not 403 (op IS blocked; status/log wrong) (`api.rs`).
- **S-H5 / A-M5** No rate limiting on `/mcp` or `/v1/synapse2` — fanout/SSH/FD exhaustion. Add `tower` limit layer.

Architecture / Quality / Perf:
- **A-C1** Domain services import the MCP protocol layer (`use crate::mcp::help`) — dependency inversion; move help to `src/help.rs`.
- **C-1 / P-C1** SSH pool fragmentation: 3 pools/process → up to 3N connections per host. Wire one shared `Arc<SshPool>`.
- **P-C2** Fanout semaphore acquired after `op()` launch — N tasks burst before the cap (`fanout.rs:209`).
- **A-H1 / D-C2** REST exposes ~13–14 of 59 actions while `openapi.json`/API.md advertise full parity — contract trap.
- **A-H3 / S-M6** `HostProtocol::Http/Https` dead but valid in config → silent SSH routing.
- **P-H1** `load_hosts()` synchronous disk + SSH-config parse on every fanout request — blocks a Tokio worker. Cache it.
- **P-H2/P-H3/P-H4** Sequential host probe (`find_host_op`), sequential per-container stats, and a full daemon-ID dedup fanout before every op — all serialize avoidable latency.

Testing / Docs / Standards:
- **T-H1** Flux driver modules (1,081 LOC) have **zero tests** — largest coverage gap.
- **T-H2 / S-M4** `scp_arg` passes unvalidated `ssh_user` to scp → ProxyCommand injection via user-controlled host config; untested.
- **D-C1** README scope table: 9 actions documented `synapse:read` but enforced `synapse:write` (operators will issue broken RBAC).
- **D-H3** Undisclosed synapse-mcp parity gaps (claude/channel notifications, templated MCP resources, root-login gate, `SYNAPSE_EXCLUDE_HOSTS`, `ALLOW_YOLO`, `DEBUG_ERRORS`, TOFU store) despite "full parity" claim.
- **B-C1** Cargo `edition = "2021"` (CLAUDE.md says 2024) — intent drift; set 2024 + `rust-version=1.85` + `cargo fix --edition`. *(Right-sized from agent's "Critical": compiles fine; not a runtime risk.)*
- **B-C2** `async_trait` on 5 trait families — migrate to native AFIT/`trait-variant` (per-call boxing).
- **B-H4** Duplicate `reqwest` 0.12+0.13 (+55 dup crates) — bump to align with lab-auth.
- **B-H5 / O-H1** Runtime image installs full `docker.io` daemon (~100-200MB) for nothing — remove.
- **B-H6** No `[profile.release]` LTO/codegen/strip tuning.
- **O-H2..O-H6** Publish/release: no concurrency cancel; unpinned `FROM`; Trivy scans `:latest` not artifact; no coverage gate; `install.sh` no checksum verify (SHA256SUMS already generated).

### Medium Priority (P2 — Plan for Next Sprint)

- **Security:** S-M1 remote symlink TOCTOU (local-only `validate_safe_path`); S-M2 `find` pattern null/length guards; S-M3 empty `scout_read_roots` footgun (safe but unhelpful error); S-M5 known_hosts wildcard warned-not-enforced; S-M7 `/status`+`/openapi.json` unauthenticated enumeration.
- **Architecture:** A-M1 half-wired `scaffold_intent`; A-M2 flat JSON schema (no `oneOf` discriminant); A-M3 fragile `trim_start_matches` REST dispatch; A-M4 `std::sync::Mutex<Instant>` in async pool (= P-M5/B-H2).
- **Performance:** P-M1 fanout error stringification; P-M2 label alloc; P-M3 emit clones; P-M4 response-cap linear scan + key clone; P-M6 logs buffered before grep; P-M7 full dmesg over SSH; P-M8 `HostConfig` clone per task; P-M9 SSH checkout double-connect race.
- **Quality:** M-2 dead comments; M-3 `let _ = state` suppression; M-4 REST inlined scout defaults (MCP/REST divergence); M-6 emit clone; M-7 TEMPLATE comments.
- **Testing:** T-M1..T-M5 (journalctl/symlink/fanout-ordering/pool-race/DenyConfirm negative tests).
- **Docs:** D-H1 unused `SYNAPSE_API_URL/KEY` documented; D-H2 CLAUDE.md env table omits ~17 vars; D-H4 module maps miss ~20 modules; D-M1..D-M5 (TEMPLATE residue, [0.1.0] example artifacts, MCP_SCHEMA scope footnote, fictional URLs, `AppState` snippet drift).
- **Standards/CI:** B-M1..B-M10 (template description, thiserror, `#[non_exhaustive]`, biome schema, rust-toolchain.toml, React 19 `forwardRef`/`use client`/imports); O-M1..O-M8 (dependabot/branch-protection, `0.0.0.0` prod bind, base pin, release permissions, JSON logging, entrypoint `set -euo pipefail`, wrong local config.toml port, metrics endpoint).

### Low Priority (P3 — Track in Backlog)

Quality L-1..L-5; Perf P-L1..P-L6; Security S-L1..S-L5 (charset friction, dmesg hint, ControlMaster symlink, exec arg limits, AuthConfig Debug redaction); Docs D-L1..D-L4; Standards B-L1..B-L8; CI O-L1..O-L6 (MSRV runs no tests, permissions blocks, rmcp major bumps, advisory re-eval date, install.sh JSON parsing, SBOM/provenance). See phase files for the full list with file:line and fixes.

## Findings by Category

| Category | Critical | High | Medium | Low |
|---|---|---|---|---|
| Code Quality | 1 | 3 | 7 | 5 |
| Architecture | 1 | 3 | 5 | 4 |
| Security | 1 | 5 | 7 | 5 |
| Performance | 2 | 4 | 9 | 6 |
| Testing | 1 | 4 | 5 | 3 |
| Documentation | 2 | 4 | 5 | 4 |
| Best Practices (lang/web/build) | 2 | 6 | 10 | 8 |
| CI/CD & DevOps | 2 | 6 | 8 | 6 |

*(Counts are per-lens distinct findings; many overlap across lenses — e.g. SSH-pool fragmentation appears as C-1/P-C1, the `Mutex<Instant>` as A-M4/P-M5/B-H2. De-duplicated, there are ~40 unique issues.)*

## Recommended Action Plan

1. **Security quick wins (small, high value):** S-H4 add 403 arm; S-H3 validate journalctl args; S-M4/T-H2 validate `ssh_user`; S-L5 redact `AuthConfig` Debug; S-H2 move ControlMaster dir off `/tmp`. *(small)*
2. **Fix the real defect:** wire transport-death eviction in drivers + test (T-H3). *(small-medium)*
3. **Harden the release pipeline:** SHA-pin actions (O-C1), stop direct-push-to-main (O-C2), activate install.sh checksum (O-H6), remove `docker.io` (O-H1/B-H5). *(medium, mostly config)*
4. **Deployment safety:** document `SYNAPSE_NOAUTH` contract + default prod compose to `127.0.0.1` (S-H1/O-M2); add rate limiting (S-H5). *(small-medium)*
5. **Doc accuracy:** fix README scopes (D-C1), API.md parity claim (D-C2/A-H1), env tables (D-H1/D-H2), parity gaps (D-H3). *(small)*
6. **Architecture/perf:** share one SSH pool (C-1/P-C1), move `mcp::help` to domain (A-C1), fix fanout semaphore placement (P-C2), cache `load_hosts` + daemon IDs + concurrent host probe (P-H1/P-H2/P-H4). *(medium)*
7. **Template detox + standards:** edition 2024 + description + rust-toolchain.toml (B-C1/B-M1/B-M7), strip TEMPLATE comments (M-7/D-M1/B-M2), `[profile.release]` (B-H6), React 19 modernization (B-M8..B-M10). *(medium)*
8. **Test backlog:** driver tests (T-H1), security negative tests, parity-test CI gate (T-H4). *(medium-large)*
9. **Backlog:** all P2/P3 items; `thiserror`/`#[non_exhaustive]`/AFIT migration; metrics endpoint; JSON logging.

## Review Metadata

- Review date: 2026-06-15
- Commit: `a2294fe`
- Phases completed: 1–5 (10 specialist agents)
- Flags: security-focus, performance-critical, strict-mode
- Output: `.full-review/00-scope.md` … `05-final-report.md`; prior artifacts archived to `.full-review/_archive-20260615-102246/`
- Verified-safe (raised, not vulnerable): no shell injection; deny-by-default read roots; non-loopback destructive-override refused; fail-closed scopes; `cargo deny` clean except S-C1.
