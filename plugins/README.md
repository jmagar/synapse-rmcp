# plugins

Claude Code and Codex plugin packages for the MCP server. Both platforms share the same skills and MCP connection config — only the manifests differ.

## Structure

```
plugins/synapse2/
├── .claude-plugin/
│   └── plugin.json       # Claude Code manifest
├── .codex-plugin/
│   ├── plugin.json       # Codex manifest
│   └── README.md         # Codex manifest field reference
├── .mcp.json             # Shared MCP server connection config
├── hooks/
│   ├── hooks.json        # Lifecycle hook definitions
│   └── plugin-setup.sh  # Deployment and validation script
└── skills/
    ├── synapse2/
    │   └── SKILL.md      # Tool documentation for Claude and Codex
    └── scaffold-project/
        └── SKILL.md      # Turns scaffold_intent JSON into an approval-first plan
```

---

## Manifests

### `.claude-plugin/plugin.json`

Claude Code plugin manifest. Defines the plugin identity, MCP server connection, lifecycle hooks, and user-configurable options.

**User config fields** (set via Claude Code plugin settings):

| Field | Type | Description |
|---|---|---|
| `server_url` | string | MCP HTTP server base URL (default: `http://localhost:40080`) |
| `api_token` | string (sensitive) | Bearer token for auth |
| `no_auth` | boolean | Disable auth (loopback dev only; non-loopback requires an upstream gateway) |
| `auth_mode` | string | `bearer` or `oauth` |
| `public_url` | string | Public URL for OAuth callbacks |
| `google_client_id` | string (sensitive) | Google OAuth client ID |
| `google_client_secret` | string (sensitive) | Google OAuth client secret |
| `auth_admin_email` | string | OAuth admin email |

### `.codex-plugin/plugin.json`

Codex equivalent of the Claude Code manifest. Shares `.mcp.json` and `skills/` with the Claude plugin. Adds Codex-specific UI fields under `interface`:

- `displayName`, `shortDescription`, `longDescription` — registry presentation
- `defaultPrompt` — three sample prompts shown in the Codex UI
- `brandColor` — hex color for the plugin icon (e.g., `#6366F1`)
- `composerIcon`, `logo` — asset paths (512×512 PNG, SVG)

See `.codex-plugin/README.md` for a full field reference and `brandColor` guide.

### `.mcp.json`

Shared MCP server connection config used by both plugins. Points both clients at the same HTTP endpoint with the same auth headers.

```json
{
  "mcpServers": {
    "synapse": {
      "type": "http",
      "url": "${user_config.server_url}/mcp",
      "headers": {
        "Authorization": "Bearer ${user_config.api_token}"
      }
    }
  }
}
```

---

## Hooks

### `hooks/hooks.json`

Defines two lifecycle hooks:

| Hook | Trigger | Script |
|---|---|---|
| `SessionStart` | Every Claude Code session start | `hooks/plugin-setup.sh` |
| `ConfigChange` | User updates plugin settings | `hooks/plugin-setup.sh` |

Timeout: 300 seconds.

### `hooks/plugin-setup.sh`

The lifecycle adapter. Runs on every session start and config change.

- Reads `CLAUDE_PLUGIN_OPTION_*` env vars from plugin `userConfig`
- Exports those values as the binary's runtime environment variables
- Prepares the plugin appdata directory
- Ensures `synapse` is available on `PATH`
- Calls `synapse setup plugin-hook "$@"`

Deployment policy, repair behavior, and failure classification live in the Rust binary, not in the hook script. The script is idempotent and intentionally does not manage Docker, systemd, config rewrites, port conflicts, or OAuth redirect construction itself.

---

## Skills

### `skills/synapse2/SKILL.md`

Three-tier structured documentation for the Synapse2 `flux` and `scout` MCP tools, used by Claude Code and Codex to understand when and how to invoke them.

**Tier 1** (above the fold): tool name, quick action table, most common usage.  
**Tier 2**: full action reference — parameters, types, example calls, response shapes.  
**Tier 3**: multi-step workflows demonstrating real-world use.

Also includes HTTP fallback examples using `CLAUDE_PLUGIN_OPTION_SERVER_URL` and `CLAUDE_PLUGIN_OPTION_API_TOKEN` env vars for when the MCP connection isn't available.


---

## Versioning

Plugin manifests intentionally do not contain a `version` field. Marketplace
versions are derived from git commits; release version synchronization is owned
by `Cargo.toml`, the npm launcher package, and the release manifest.

---

## Maintenance checklist

1. Keep Claude, Codex, and Gemini manifests pointed at the same Synapse2 server.
2. Keep `skills/synapse2/SKILL.md` aligned with the canonical operation registry.
3. Preserve the no-`version` manifest contract.
4. Run `scripts/validate-plugin-layout.sh` after plugin changes.
