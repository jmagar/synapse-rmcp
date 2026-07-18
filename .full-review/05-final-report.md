# Comprehensive Code Review Report

## Review Target

The entire tracked `synapse2` repository at baseline commit `dc75a01763334549e238061e49fdc2056fa5cec8`: Rust service/CLI/MCP/REST, SSH and Docker execution, browser UI, tests, documentation, installers, packaging, CI/CD, and deployment configuration.

## Executive Summary

The repository has broad automated coverage and a generally well-separated domain architecture, but several trust and resource boundaries are documented or tested more strongly than they are enforced. The most urgent issues are unauthenticated trusted-gateway exposure, command-argument and remote-symlink escapes, unbounded subprocess/filesystem/request admission, an unusable production browser-auth contract, and operational automation that can publish or act on the wrong artifact/workload.

The eight phase reports produced 76 actionable findings. After folding duplicate architecture/performance findings and treating missing tests/docs as required acceptance criteria for their underlying defects, they resolve into 60 remediation items: 0 P0, 15 P1, 29 P2, and 16 P3. Every item is in scope for immediate remediation per the user directive.

## Findings by Priority

### Critical Issues (P0 — Must Fix Immediately)

None.

### High Priority (P1 — Fix Before Next Release)

1. Enforce requested `scout exec` timeouts across CLI/MCP/domain execution and test actual cancellation.
2. Key/invalidate SSH and Docker connections by full topology identity so alias retargeting cannot reach stale hosts.
3. Implement a supported authenticated browser UI contract, states, and E2E coverage; document static bearer tokens as read-only.
4. Replace unscoped trusted-gateway bypass with enforceable peer/mTLS/auth constraints, safe startup policy, tests, and threat-boundary docs.
5. Replace name-only Scout command validation with typed per-command argument policy, blocking `rg --pre` and equivalent execution/config escapes.
6. Canonicalize and enforce remote read roots immediately before access, including intermediate symlinks and TOCTOU tests.
7. Pin and isolate the privileged OpenWiki supply chain; remove mutable npm/action execution from privileged network/PR context.
8. Bound stdout/stderr while draining local and SSH producers rather than truncating only after full buffering.
9. Bound file sizes and replace Scout delta's quadratic full-file diff with a bounded off-runtime algorithm.
10. Replace the HTTP semaphore's unbounded waiter queue with bounded admission/load shedding and overload tests.
11. Bound `find`/tree traversal at the producer and terminate once result/byte limits are reached.
12. Correct repair automation so it cannot stop/install `example-mcp`; validate and test Synapse targets.
13. Scan the exact container digest before publication, preserve SARIF on failure, and publish only verified artifacts.
14. Make production Compose consume `ghcr.io/jmagar/synapse2` (preferably by digest) and contract-test the resolved image.
15. Add all missing high-value regression suites for the above trust/resource/installer/browser boundaries; tests must fail on the baseline defects and pass after remediation.

### Medium Priority (P2 — Address in This Remediation)

1. Preserve Scout CLI argv for `exec`/`emit` and reject unknown/duplicate flags.
2. Reject invalid numeric CLI values instead of silently substituting defaults.
3. Propagate primary/fallback log command failures with bounded context.
4. Reconcile web destructive-action requirements (`force`, `prune_target`) with backend validation.
5. Propagate unexpected container-stop failures before recreate mutation.
6. Establish one canonical dotted REST/action contract for help, parsing, OpenAPI, web metadata, scope, and destructiveness.
7. Wire SSH eviction, Docker cache clearing, forwarded-socket cleanup, and graceful shutdown into HTTP/stdio runtime ownership.
8. Apply canonical read-root/sensitive-path policy to filesystem operands of generic Scout commands.
9. Make npm/shell installers require HTTPS, verified checksums/attestations, bounded downloads, safe tar entries, and fail-closed cleanup; add security tests.
10. Resolve or time-bound the Marvin RSA advisory exception and migrate signing when a constant-time path is available.
11. Disable cwd dotenv loading in production or require an explicit secure file/flag with ownership checks.
12. Fetch all-container stats with bounded concurrency and preserve per-container errors.
13. Add Compose-discovery single flight and bounded/batched remote reads.
14. Resolve hostless container ownership with bounded first-success concurrency.
15. Correct Docker bind/publish documentation and document `SYNAPSE_MCP_BIND_HOST` separately from container bind.
16. Mechanically synchronize CONFIG/ENV documentation, including concurrency and OAuth TTL/rate/static-token behavior.
17. Remove invented container auto-bind behavior from deployment docs.
18. Expand web/backend contract tests beyond IDs and run `pnpm test` in CI.
19. Load one canonical Aurora token stylesheet and guard all referenced CSS variables.
20. Pin every release/npm third-party GitHub Action to full SHAs and enforce the standard.
21. Use one explicit production Rust toolchain across CI, releases, and container builds; keep MSRV separate.
22. Move local filesystem work off Tokio executor threads.
23. Replace template identifiers in runtime-freshness verification and test systemd/container detection.
24. Build, resolve, and smoke the deployable container on pull requests.
25. Restore audited entrypoint permission repair or parameterize/preflight runtime UID/GID; test non-1000 hosts.
26. Repair the release-readiness gate to validate semantic artifacts and run it in CI.
27. Fix auth-smoke variables/URL/port/temp-file handling and test the documented command verbatim.
28. Separate `/health` liveness from bounded `/ready` dependency/config readiness; update monitoring and docs.
29. Implement persistent size-aware log rotation/retention and diagnostics.

### Low Priority (P3 — Address in This Remediation)

1. Make `MemoryCache` deliver documented LRU/strict-capacity behavior, including zero and concurrent inserts, with a controllable clock.
2. Prevent facade dependency mutators from violating the shared SSH-pool invariant.
3. Split `src/actions/flux.rs` below the repository's 400-line advisory using sibling modules.
4. Update Vite to a release containing fixes for the two Windows dev-server advisories.
5. Remove yanked `spin 0.9.8` while retaining the deny gate.
6. Pass release metadata through environment variables and validate tags before privileged shell use.
7. Lazily materialize bounded fanout work instead of one future/config clone per host.
8. Cache revisioned topology snapshots and reload blocking sources only when changed.
9. Add enforceable coverage/live JSON-RPC/SSH integration gates rather than optional/silent-skipping paths.
10. Correct runtime image docs to describe Bollard rather than a nonexistent Docker CLI.
11. Remove the unnecessary client boundary from the static API page.
12. Remove seven unused Radix dependencies and add dependency-hygiene automation.
13. Remove or modernize the unused blocking legacy Docker CLI module.
14. Make web health polling abortable and completion-driven.
15. Repair standalone CLI generation to use Synapse identity and a valid MCP handshake, with smoke coverage.
16. Fold cache/lifecycle/recreate/CLI/concurrency test-quality gaps into deterministic regression suites using controllable clocks and scriptable fakes.

## Findings by Category

- **Code Quality:** 8 findings (0 Critical, 1 High, 6 Medium, 1 Low)
- **Architecture:** 7 findings (0 Critical, 2 High, 2 Medium, 3 Low)
- **Security:** 11 findings (0 Critical, 4 High, 4 Medium, 3 Low)
- **Performance:** 10 findings (0 Critical, 4 High, 4 Medium, 2 Low)
- **Testing:** 13 findings (0 Critical, 6 High, 5 Medium, 2 Low)
- **Documentation:** 8 findings (0 Critical, 3 High, 4 Medium, 1 Low)
- **Best Practices:** 8 findings (0 Critical, 0 High, 4 Medium, 4 Low)
- **CI/CD & DevOps:** 11 findings (0 Critical, 3 High, 7 Medium, 1 Low)

## Recommended Action Plan

1. **Security/runtime lane (large):** gateway/auth, command operands, remote containment, bounded output/find/delta/admission, connection identity/lifecycle, readiness, and regression tests.
2. **CLI/contracts/web/docs lane (large):** CLI correctness, canonical REST/web metadata, browser auth/Aurora, operator documentation, module split, and web tests.
3. **CI/install/operations/performance lane (large):** installer provenance, workflow pinning/scanning, release/runtime scripts, Compose/image/UID behavior, log rotation, bounded remote concurrency, and CI gates.
4. Integrate lanes, run all Rust/web/npm/docs/policy/container quality gates, then open a PR.
5. Run `lavra-review` on the PR, file and fix every P0-P3 finding, rerun gates, and push final amendments.

## Review Metadata

- Review date: 2026-07-18
- Baseline: `dc75a01763334549e238061e49fdc2056fa5cec8`
- Phases completed: Code Quality & Architecture; Security & Performance; Testing & Documentation; Best Practices & Standards; Consolidated Report
- Flags: entire repository; Rust/Axum/rmcp plus Next.js/React; no strict/security-only/performance-only narrowing
- Baseline verification: `cargo test --locked` passed (667 unit tests plus all integration/doc suites); generated schema/OpenAPI checks passed; `cargo deny check` failed only on yanked `spin 0.9.8`.
