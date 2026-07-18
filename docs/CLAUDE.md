---
title: "Documentation Instructions"
doc_type: "guide"
status: "active"
owner: "synapse2"
audience:
  - "contributors"
  - "agents"
scope: "service"
source_of_truth: false
upstream_refs:
  - "src/actions.rs"
  - "src/config.rs"
  - "docs/PATTERNS.md"
last_reviewed: "2026-06-13"
---

# Documentation Instructions

This directory contains guides, reference material, generated contracts, and
working records for the Synapse2 Rust MCP server.

Both humans and agents operate this codebase. Write docs, contracts, specs,
examples, and commands assuming both audiences. Prefer structured, runnable, and
self-contained content. Avoid prose that only makes sense in the context of a
prior conversation.

---

## Documentation Layers

Use the right layer for the job:

- `docs/*.md` — Stable service guides: setup, architecture, API, deployment,
  testing, and operations.
- `docs/PATTERNS.md` — Family-level rmcp server patterns inherited from
  `rmcp-template`. Keep illustrative `example` / `EXAMPLE_*` snippets there
  when they describe how to adapt a new server.
- `docs/generated/` — Machine-produced contracts committed for compatibility
  checks, especially OpenAPI output.
- `docs/contracts/` — Durable JSON schemas and example payloads.
- `docs/specs/` — Design specs and handoff docs for protocol or workflow
  features.
- `docs/plans/`, `docs/reports/`, `docs/research/`, `docs/sessions/` —
  working artifacts. Promote accepted requirements from these into stable docs.

---

## Files in This Directory

| File | Purpose | Update when |
|---|---|---|
| `README.md` | High-level documentation index | Adding, removing, or renaming docs |
| `QUICKSTART.md` | Local smoke test path | Startup sequence, CLI flags, port, or auth env changes |
| `API.md` | MCP, CLI, and REST action reference | Actions, parameters, scopes, or parity behavior changes |
| `MCP_SCHEMA.md` | Generated MCP schema contract | `OPERATION_SPECS` changes |
| `AUTH.md` | Bearer/OAuth/auth-policy behavior | Auth mode, scope, token, or gateway behavior changes |
| `CONFIG.md` / `ENV.md` | Config loading and environment variables | `src/config.rs` or host-config loading changes |
| `ARCHITECTURE.md` | Module map and layering | Service/module boundaries change |
| `DEPLOYMENT.md`, `DOCKER.md`, `SYSTEMD.md` | Runtime deployment guides | Container, systemd, port, or appdata behavior changes |
| `PLUGINS.md` | Claude/Codex/Gemini plugin packaging | Plugin manifests or hook scripts change |
| `PATTERNS.md` | rmcp-family conventions | A reusable family pattern changes |
| `CLAUDE.md` (this file) | Instructions for agents and contributors navigating docs | Directory structure or doc authority changes |

---

## References

`docs/references/` is intentionally gitignored and populated by
`scripts/refresh-docs.sh` when upstream MCP or registry docs need to be captured.
Prefer captured references before raw web search for protocol behavior, but
verify upstream when a spec area is fast-moving or marked preview.

Do not treat seed transcripts or conversation context as sufficient evidence for
what the spec requires. If spec behavior matters, cite the reference file or the
current upstream source.

---

## Naming

- Current binary: `synapse`
- Current repo/service: `synapse2`
- Current env prefix: `SYNAPSE_*` and `SYNAPSE_MCP_*`
- Current REST action endpoint: `POST /v1/synapse2`
- Current MCP tools: `flux` and `scout`

Do not add new `SYNAPSE2_*`, `EXAMPLE_*`, `example-mcp`, or `/v1/example`
references to service docs. Those names may appear in `docs/PATTERNS.md` only
when they are intentionally documenting reusable template-family examples.

---

## Source of Truth

- Actions, scopes, and MCP schema: `src/actions.rs`, `src/mcp/schemas.rs`, and
  `docs/MCP_SCHEMA.md` after regeneration.
- CLI flags: `src/cli.rs` and live `synapse --help` output.
- Config and env vars: `src/config.rs`, `src/host_config.rs`, and `.env.example`.
- Agent memory files: root `CLAUDE.md`; `AGENTS.md` and `GEMINI.md` must be
  symlinks to it. The same rule applies in `docs/`, `apps/web/`, and
  `plugins/synapse2/`.

After adding any new `CLAUDE.md` anywhere in the repo, regenerate the symlinks:

```bash
just symlink-docs
# or: cargo xtask symlink-docs
```

---

## Style

- Keep examples runnable as written. Verify port numbers, command names, and
  flag names against the code before committing.
- Keep historical or generated material out of stable guides unless distilled
  into current guidance.
- When a doc summarizes code, link back to the code path in frontmatter or text.
- When a change touches actions, run `just schema-docs` and update schema docs.
- When a change touches deployment or runtime env, update both `docs/ENV.md` and
  `.env.example` if applicable.
