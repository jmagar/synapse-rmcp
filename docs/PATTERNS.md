# Synapse2 Patterns

Synapse2 follows the rmcp-server family patterns maintained upstream in:

```text
/home/jmagar/workspace/rmcp-template/docs/PATTERNS.md
```

Do not copy the full family catalog back into this repo. Update the upstream
`rmcp-template` catalog when a reusable pattern changes, then keep this file to
Synapse2-specific decisions and local exceptions.

## Local Rules

- Business actions keep MCP + CLI parity. REST remains a thin compatibility
  endpoint at `POST /v1/synapse2`.
- `flux` actions live on `FluxService`; `scout` actions live on `ScoutService`.
  `SynapseService` stays a thin facade.
- Production Rust modules should stay below the soft line budget when a focused
  sibling split is natural. Use `foo.rs` plus `foo/` submodules, never `mod.rs`.
- Root and nested agent memory files use `CLAUDE.md` as source of truth;
  `AGENTS.md` and `GEMINI.md` must be symlinks to it.
- Plugin manifests do not carry explicit `version` fields.
- Plugin setup hooks delegate to `<binary> setup plugin-hook` for repair mode
  and `<binary> setup plugin-hook --no-repair` for audit mode. The JSON contract
  includes `exit_policy`, `blocking_failures`, `advisory_failures`, and
  `ran_repair`.

## Current Local Exceptions

- `scaffold_intent` is MCP-only because it depends on elicitation and skill
  handoff semantics that do not translate to a one-shot CLI command.
- `serve`, `mcp`, `doctor`, `watch`, and `setup` are operational CLI commands,
  not MCP business actions.
- `docs/sessions/` and `.full-review/` artifacts are historical working records;
  promote accepted requirements into stable docs before treating them as current
  guidance.

## Validation

Run the local pattern gate after changing architecture, docs conventions, plugin
packaging, or action surfaces:

```bash
cargo xtask patterns
```
