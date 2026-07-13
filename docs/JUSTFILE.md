---
title: "Justfile"
doc_type: "guide"
status: "active"
owner: "synapse2"
audience:
  - "contributors"
  - "agents"
scope: "synapse2"
source_of_truth: false
last_reviewed: "2026-06-12"
---

# Justfile

`Justfile` is the operator command surface for local development, CI parity,
Docker, plugin packaging, and diagnostics. Run `just --list` for the complete
current recipe list.

## Core development recipes

| Recipe | Purpose |
|---|---|
| `just dev` | Run HTTP MCP server on loopback in no-auth dev mode (`SYNAPSE_MCP_NO_AUTH=true`). |
| `just mcp` | Run stdio MCP transport (`cargo run -- mcp`). |
| `just greet` | Quick CLI smoke test (`cargo run -- flux host status`). |
| `just doctor` | Pre-flight environment/config checks (`cargo run -- doctor`). |
| `just build` / `just build-release` | Debug/release Rust builds. |
| `just build-web` | Build static Next.js web assets (`apps/web/out`). |
| `just build-full` | Build web assets then release binary. |
| `just gen-token` | Generate a random bearer token for `SYNAPSE_MCP_TOKEN`. |

## Quality gates

| Recipe | Purpose |
|---|---|
| `just verify` | `fmt-check` + `lint` + `check` + `test`. |
| `just template-check` | Pattern, module-size, plugin, schema, OpenAPI, scaffold, and template feature checks. |
| `just pre-release` | Release-readiness gate (`scripts/pre-release-check.sh`). |
| `just fmt` / `just fmt-check` | Format Rust or check formatting. |
| `just lint` | `cargo clippy --all-targets -- -D warnings`. |
| `just check` | `cargo check`. |
| `just test` / `just test-ci` | Run cargo-nextest locally or with CI profile. |
| `just schema-docs-check` | Verify MCP schema contract docs. |
| `just openapi-check` | Verify generated OpenAPI docs for `/v1/synapse2`. |
| `just module-size-check` | Enforce Rust module size budgets. |

## Deployment recipes

| Recipe | Purpose |
|---|---|
| `just docker-build` | Build Docker image `synapse2`. |
| `just docker-up` / `just docker-down` | Start/stop compose stack. |
| `just docker-rebuild` | Rebuild image and recreate Docker service. |
| `just docker-logs` | Follow container logs. |
| `just runtime-current` | Detect stale running runtime against `target/release/synapse`. |
| `just auth-smoke` | Test bearer auth path against a running server. |
| `just test-mcporter` | Run live MCP integration tests. |
| `just repair` | Rebuild and restart via systemd or Docker when available. |
| `just install-local` | Install `target/release/synapse` to `~/.local/bin/synapse`. |

## Plugin and xtask recipes

| Recipe | Purpose |
|---|---|
| `just build-plugin` | Copy release binary to `bin/synapse` and `plugins/synapse2/bin/synapse`. |
| `just sync-bin` | Explicit alias for refreshing repo and plugin binary artifacts. |
| `just validate-plugin` | Validate Claude/Codex/Gemini plugin manifests and skills. |
| `just dist` | `cargo xtask dist` build and copy release artifacts. |
| `just ci` | `cargo xtask ci` full local CI. |
| `just symlink-docs` | `cargo xtask symlink-docs` syncs `AGENTS.md`/`GEMINI.md` symlinks to `CLAUDE.md`. |
| `just check-env` | `cargo xtask check-env` validates required environment. |
| `just patterns` | `cargo xtask patterns` checks architecture contracts. |

## Reference docs

```just
refresh-docs:           bash scripts/refresh-docs.sh
refresh-docs-repomix:   bash scripts/refresh-docs.sh --skip-crawl
refresh-docs-crawl:     bash scripts/refresh-docs.sh --skip-repomix
refresh-docs-dry:       bash scripts/refresh-docs.sh --dry-run
```

## Doctor output

Use `just doctor` or `synapse doctor` for a pre-flight report. The check covers:

- Config and appdata paths.
- Binary visibility on `PATH`.
- MCP bind host/port and auth posture.
- Host topology discovery from `SYNAPSE_HOSTS_CONFIG`, `SYNAPSE_CONFIG_FILE`, or
  `~/.ssh/config`.

Exit code 0 means ready. Exit code 1 means one or more issues were found.
