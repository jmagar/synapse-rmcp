# Review Scope

## Target

Full repository review of `synapse2` at commit `a2294fe` (`ci: drop arm64 from Docker Publish to fix build timeout (#22)`), branch `claude/adoring-hellman-0dc9e5` (even with `main`). Working tree clean.

`synapse2` is the Rust MCP + CLI server for local Synapse workflows — a full-parity Rust port of `synapse-mcp`. It exposes two action-dispatched MCP tools: `flux` (Docker daemon/container/host/Compose) and `scout` (SSH/local fs, process, ZFS, logs, transfer, allowlisted exec). Binary `synapse`; HTTP MCP at `127.0.0.1:40080`; REST compat at `POST /v1/synapse2`.

Codebase size: ~34,853 lines of Rust across `src/`, `tests/`, `xtask/` (incl. `*_tests.rs` sidecars); ~2,787 lines of TypeScript/TSX in `apps/web/` (Next.js 16 static UI).

## Files

- `src/` — Rust service core (flux, scout, mcp, server, api, cli, ssh, docker_client, config, actions, formatters, logging, runtime_budget, token_limit)
- `tests/` — integration tests (cli_parse, tool_dispatch, api_routes, parity)
- `xtask/` — module-size / pattern enforcement tooling
- `apps/web/` — Next.js 16 static web UI
- `bin/`, `scripts/` — operational + lint/check scripts
- `config/`, `config.example.toml`, `config.toml`, `deny.toml` — configuration
- `plugins/synapse2/` — Claude/Codex/Gemini plugin + skill assets
- `.github/` — CI workflows
- `docs/` — API.md, MCP_SCHEMA.md, CONFIG.md, ENV.md, etc.
- Root manifests / ops: `Cargo.toml`, `Cargo.lock`, `Justfile`, `lefthook.yml`, `docker-compose*.yml`, `Dockerfile`/`entrypoint.sh`, `install.sh`, `CHANGELOG.md`, `README.md`, `server.json`

## Flags

- Security Focus: **yes**
- Performance Critical: **yes**
- Strict Mode: **yes**
- Framework: Rust MCP server (rmcp, Axum, Tokio, bollard Docker, SSH, lab-auth) + Next.js 16 static web UI

## Review Phases

1. Code Quality & Architecture
2. Security & Performance
3. Testing & Documentation
4. Best Practices & Standards
5. Consolidated Report

## Notes from prior artifacts (archived)

A prior review run was archived to `.full-review/_archive-20260615-102246/` (top-level 00–05, per-component `api/cli/mcp/web/docs` subdirs, and `synapse-mcp-gap-review.md`). Not used as input; this is a fresh session.
