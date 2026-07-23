---
date: 2026-07-23 16:18:43 EST
repo: git@github.com:jmagar/synapse.git
branch: main
head: 9c2dbf5e999676534eb34337e06560e021f6710c
session id: 019f8d88-83b4-7e91-8d63-8b97c6dfdf79
transcript: /home/jmagar/.codex/sessions/2026/07/23/rollout-2026-07-23T01-52-41-019f8d88-83b4-7e91-8d63-8b97c6dfdf79.jsonl
working directory: /home/jmagar/workspace/synapse
worktree: /home/jmagar/workspace/synapse
---

# Synapse runtime configuration audit

## User Request

Ensure this Rust service has complete, correctly located environment and TOML configuration.

## Session Overview

Synapse runtime appdata was corrected from legacy `~/.synapse` to the code-defined `~/.synapse2` location. A minimal valid TOML and complete env were installed, the Compose override remounted `/data` from `~/.synapse2`, and a topology read passed.

## Sequence of Events

1. Compared loader defaults, tracked scaffold config, Compose mount, and live container.
2. Copied the complete env into `~/.synapse2` and created a minimal valid MCP TOML.
3. Added the appdata override with corrected `/data` volume and working directory.
4. Recreated the service and verified `synapse scout nodes`.

## Key Findings

- The old host volume used `.synapse`, while the binary's canonical directory is `.synapse2`.
- The tracked TOML was not suitable to copy directly as deployed config.

## Technical Decisions

- Corrected the mount and created only a valid non-secret `[mcp]` section.
- Preserved the former dotenv at `/home/jmagar/.config-audit-backup/20260723T022512/repo-env-files/synapse.env`.

## Files Changed

| status | path | previous path | purpose | evidence |
|---|---|---|---|---|
| created | `/home/jmagar/.synapse2/.env` | `./.env` | Canonical env | Topology read passed |
| created | `/home/jmagar/.synapse2/config.toml` | — | Valid MCP config | Parsed |
| created | `/home/jmagar/.synapse2/docker-compose.env.yml` | — | Correct `/data` mount and working dir | Docker inspect |
| renamed | `/home/jmagar/.config-audit-backup/20260723T022512/repo-env-files/synapse.env` | `./.env` | Secure old env | Mode `0600` |
| created | `docs/sessions/2026-07-23-runtime-configuration-audit.md` | — | Repo log | This file |

## Beads Activity

No bead activity observed for Synapse.

## Repository Maintenance

- Plans: no completed session plan required moving.
- Beads: read-only inspection.
- Worktrees/branches: fetched/pruned; behind local `main` was preserved.
- Stale docs: no broad documentation edit was mixed into runtime migration.
- Cleanup: no source branch was rewritten.

## Tools and Skills Used

- Loader/schema inspection, Docker Compose/inspect, TOML checks, live Synapse CLI, Git, and `vibin:save-to-md`.

## Commands Executed

| command | result |
|---|---|
| `docker compose ... config -q` | Valid |
| Docker mount inspect | `~/.synapse2 -> /data` |
| `synapse scout nodes` | Exit 0 |

## Behavior Changes (Before/After)

| area | before | after |
|---|---|---|
| Appdata mount | `~/.synapse` | `~/.synapse2` |
| Config resolution | Legacy/relative | `/data/config.toml` |

## Verification Evidence

| command | expected | actual | status |
|---|---|---|---|
| Container health | Healthy | Healthy | pass |
| Topology read | Success | Exit 0 | pass |

## Risks and Rollback

Restore the former env and original volume mapping to return to the legacy appdata path.

## Decisions Not Taken

- Did not copy the generic scaffold TOML.

## Next Steps

- Keep `~/.synapse2` as the single canonical host appdata directory.
