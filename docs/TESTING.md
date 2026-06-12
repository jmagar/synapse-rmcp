---
title: "Testing"
doc_type: "guide"
status: "active"
owner: "rmcp-template"
audience:
  - "contributors"
  - "agents"
scope: "template"
source_of_truth: false
upstream_refs:
  - "docs/PATTERNS.md"
last_reviewed: "2026-05-15"
---

# Testing

The test strategy is layered: parse at the CLI layer, test business/service behavior without a server, then smoke-test live MCP HTTP with mcporter.

## Rust tests

```bash
cargo nextest run
cargo nextest run --profile ci
cargo test
just test-ci
```

All repos use `cargo nextest` instead of `cargo test`. Configure in `.config/nextest.toml`:

```toml
[profile.default]
fail-fast = false

[profile.ci]
fail-fast = true
retries = 2
```

## Key test files

| File | Purpose |
|---|---|
| `tests/cli_parse.rs` | CLI parser behavior. |
| `tests/tool_dispatch.rs` | Service/action semantics without live credentials. |
| `tests/api_routes.rs` | REST and mounted auth route behavior. |
| `tests/plugin_contract.rs` | Plugin package and hook contracts. |
| `tests/template_invariants.rs` | Automation/template invariants. |
| `src/app_tests.rs` | Private service-layer unit tests (sidecar to `app.rs`). |

## Test sidecars

All tests that need access to private functions live in `_tests.rs` sidecar files, not inline:

```rust
// src/app.rs
pub struct SynapseService { ... }
impl SynapseService { ... }

#[cfg(test)]
#[path = "app_tests.rs"]
mod tests;

// src/app_tests.rs
use super::*;  // access to private items

#[test]
fn destructive_gate_blocks_without_confirm() {
    let svc = SynapseService::new();
    let err = svc.destructive_gate(false).unwrap_err();
    assert!(err.to_string().contains("confirm=true"));
}

#[test]
fn destructive_gate_allows_with_confirm() {
    let svc = SynapseService::new();
    assert!(svc.destructive_gate(true).is_ok());
}
```

## Test helpers

`src/lib.rs` exports helpers for integration tests:

```rust
#[cfg(any(test, feature = "test-support"))]
pub mod testing {
    pub fn loopback_state() -> AppState {
        AppState {
            config: McpConfig::default(),
            auth_policy: AuthPolicy::LoopbackDev,
            service: stub_service(),
        }
    }

    fn stub_service() -> SynapseService {
        SynapseService::new()
    }
}
```

Use `loopback_state()` in integration tests:

```rust
// tests/tool_dispatch.rs
use example_mcp::testing::loopback_state;

#[tokio::test]
async fn help_returns_help_key() {
    let state = loopback_state();
    let result = execute_tool(&state, "example", json!({"action": "help"})).await.unwrap();
    assert!(result.get("help").is_some());
    assert!(!result["help"].as_str().unwrap().is_empty());
}
```

## Live MCP tests

```bash
just dev
bash tests/mcporter/test-mcp.sh
just test-mcporter
```

The mcporter harness validates tools and resources against a running server. It logs calls to `/tmp/test-mcp.<timestamp>.log`.

The test script validates:
- auth rejection when `SYNAPSE_MCP_TOKEN` is set
- read-only `flux`/`scout` action behavior and help
- MCP resource behavior for `synapse://schema/flux` and `synapse://schema/scout`

Use semantic assertions, not liveness-only checks:

```bash
# Bad test — only proves MCP responded
run_test "server info" "scout" '{"action":"nodes"}'

# Good test — proves the service actually returned real data
run_test "nodes has hosts" "scout" '{"action":"nodes"}' "hosts"
```

## Template checks

```bash
just template-check
cargo xtask patterns
scripts/pre-release-check.sh
```

## Principles

- Assert semantic values, not just valid JSON.
- Assert defaults explicitly.
- Keep business logic tests below HTTP when possible.
- Use live mcporter tests for transport/resource/auth integration.
- A test that checks `is_error: false` only verifies the protocol layer responded — prove the actual data is correct.

See `docs/PATTERNS.md` §12, §17, §24 for test sidecar, mcporter, and nextest patterns.
