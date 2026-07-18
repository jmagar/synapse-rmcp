# Phase 1: Code Quality & Architecture Review

## Code Quality Findings

### High

1. **`scout exec` accepts but discards `timeout_secs`.** `src/actions/scout.rs:181-188`, `src/actions/dispatch.rs:101-105`, `src/scout_service.rs:160-176`, and `src/cli/scout.rs:82-93,281-285` parse the field but never pass it into execution, so callers can block until the global five-minute deadline. Thread it through both transports and the domain service, and test that it is enforced.

### Medium

2. **Scout CLI silently discards command arguments.** `src/cli/help/catalog.rs:256-261` advertises `--args`, while `src/cli/scout.rs:82-93,115-123` constructs empty argument vectors for `exec` and `emit`. Implement repeatable/delimited argv parsing and parity tests.
3. **Generic CLI parsing silently accepts unknown flags.** `src/cli.rs:300-326` ignores well-formed option pairs that are not the flag currently being queried. Parse once against an explicit allowed set and reject unknown/duplicate options.
4. **Invalid numeric Scout CLI values silently become defaults.** `src/cli/scout.rs:30-32,42-45,55-57,83-84,115-116,165-167,184-189` uses `parse().unwrap_or(default)`. Return flag-specific validation errors using the established Flux parser pattern.
5. **Log retrieval converts command failures into successful empty output.** `src/scout_service/logs.rs:391-417` accepts nonzero primary/fallback exits except a narrow missing-file case. Check both statuses and return bounded contextual errors.
6. **Web action metadata has drifted from backend validation.** `apps/web/lib/template.ts:203-248`, `apps/web/app/tools/page.tsx:169-199`, and `src/actions/flux.rs:257-278` disagree about required `force` and `prune_target`, leaving prune unusable. Correct metadata and add required-field/type/enum contract checks.
7. **Container recreation suppresses every stop failure.** `src/flux_service/container_lifecycle.rs:219-227` discards all stop errors rather than only the expected already-stopped condition. Propagate unexpected daemon, auth, timeout, and transport failures.

### Low

8. **`MemoryCache` does not deliver its documented LRU/strict-capacity behavior.** `src/cache.rs:42-46,92-114,132-153` evicts by insertion time, races concurrent capacity checks, and permits an entry at zero capacity. Implement true access recency with atomic bounded insertion or rename the abstraction and correct edge cases.

## Architecture Findings

### High

1. **Connection caches can permanently target stale hosts after topology changes.** `src/docker_client/cache.rs:53-69`, `src/ssh/pool.rs:191-208`, `src/host_config.rs:194-215`, and `src/flux_service.rs:114-119` key long-lived clients/sessions only by alias (and SSH port), omitting endpoint/user/credential/socket identity. Use a canonical connection key or invalidate caches when topology revisions change; test alias retargeting.
2. **The embedded operator UI is disconnected from mounted HTTP authentication.** `apps/web/lib/api.ts:60-69`, UI pages, and `src/server/routes.rs:146-174,240-249` expose unauthenticated static UI that issues protected REST calls without bearer/OAuth state. Define and implement a supported browser-auth contract and explicit authenticated/disabled states.

### Medium

3. **REST capability metadata advertises names REST rejects.** `src/actions.rs:71-218,338-347`, `src/api.rs:101-177`, and `apps/web/lib/template.ts:47-77` mix top-level action metadata with a separate dotted-action parser/catalog. Establish one canonical dotted-operation registry and generate REST help/OpenAPI/web metadata.
4. **SSH eviction and shutdown lifecycle APIs are never wired into runtime composition.** `src/ssh/pool.rs:283-337`, `src/docker_client/cache.rs:100-103`, and `src/main.rs` never start eviction or close/clear shared resources. Add runtime lifecycle ownership covering HTTP and stdio startup/shutdown.

### Low

5. **Facade dependency mutators can violate the single-SSH-pool invariant.** `src/app.rs:39-89` and `src/flux_service.rs:76-98` independently replace pool-dependent components. Use a validated dependency bundle or focused test constructors.
6. **`MemoryCache` is documented as LRU but is FIFO.** This overlaps code-quality finding 8 and should be remediated once.
7. **Flux action parsing exceeds the 400-line advisory budget.** `src/actions/flux.rs` is 432 real-code lines and combines four domains. Split into sibling `docker`, `container`, `host`, and `compose` modules behind a thin router.

## Critical Issues for Phase 2 Context

- No Critical-severity findings were reported.
- Security review should focus on stale connection identity and the web/auth trust-model mismatch.
- Performance review should focus on cache-key correctness, cache boundedness, SSH/client lifecycle, and command timeout enforcement.
- Cross-transport contract duplication is the dominant systemic cause behind several defects.

## Phase Counts

- Code Quality: 0 Critical, 1 High, 6 Medium, 1 Low.
- Architecture: 0 Critical, 2 High, 2 Medium, 3 Low (one overlaps Code Quality).
