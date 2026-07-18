# Review Scope

## Target

The entire tracked `synapse2` repository at commit `dc75a01763334549e238061e49fdc2056fa5cec8`, including the Rust service, CLI, MCP and REST transports, authentication and authorization, Docker and SSH operations, web application, packaging and installation scripts, tests, documentation, CI/CD, deployment configuration, and plugin assets.

## Files

- All 388 files tracked at the review baseline.
- Production Rust under `src/`, integration tests under `tests/`, and build tooling under `xtask/`.
- Web UI under `apps/web/`.
- npm packaging under `packages/synapse-rmcp/` and plugin assets under `plugins/`.
- CI, container, Compose, release, installer, configuration, documentation, and repository policy files.
- Prior `.full-review/` artifacts were explicitly removed before this fresh review; generated build outputs and Git internals are excluded.

## Flags

- Security Focus: no
- Performance Critical: no
- Strict Mode: no
- Framework: Rust/Axum/rmcp with Next.js/React

## Review Phases

1. Code Quality & Architecture
2. Security & Performance
3. Testing & Documentation
4. Best Practices & Standards
5. Consolidated Report
