# synapse2 Configuration

## MCP

| Variable | Default | Purpose |
|---|---|---|
| `SYNAPSE_MCP_HOST` | `127.0.0.1` | HTTP bind host |
| `SYNAPSE_MCP_PORT` | `40080` | HTTP bind port |
| `SYNAPSE_MCP_SERVER_NAME` | `synapse2` | MCP server name advertised to clients |
| `SYNAPSE_MCP_TOKEN` | unset | Static bearer token for bearer mode |
| `SYNAPSE_MCP_NO_AUTH` | false | Disable auth on loopback only |
| `SYNAPSE_MCP_ALLOW_DESTRUCTIVE` | false | Skip destructive-operation confirmation prompts |
| `SYNAPSE_MCP_MAX_CONCURRENCY` | `50` | Maximum active API/MCP requests; `0` disables the cap |
| `SYNAPSE_MCP_ALLOWED_HOSTS` | unset | Extra Host header values |
| `SYNAPSE_MCP_ALLOWED_ORIGINS` | unset | Extra CORS origins |
| `SYNAPSE_MCP_PUBLIC_URL` | unset | Public base URL for OAuth metadata |
| `SYNAPSE_MCP_AUTH_MODE` | `bearer` | `bearer` or `oauth` |
| `SYNAPSE_MCP_AUTH_ADMIN_EMAIL` | unset | OAuth admin email |
| `SYNAPSE_MCP_GOOGLE_CLIENT_ID` | unset | Google OAuth client ID |
| `SYNAPSE_MCP_GOOGLE_CLIENT_SECRET` | unset | Google OAuth client secret |
| `SYNAPSE_MCP_AUTH_SQLITE_PATH` | appdata auth DB | OAuth SQLite database path |
| `SYNAPSE_MCP_AUTH_KEY_PATH` | appdata JWT key | OAuth JWT signing key path |
| `SYNAPSE_MCP_AUTH_ALLOWED_REDIRECT_URIS` | unset | Extra OAuth redirect URI patterns |
| `SYNAPSE_MCP_AUTH_ACCESS_TOKEN_TTL_SECS` | `3600` | OAuth access-token lifetime |
| `SYNAPSE_MCP_AUTH_REFRESH_TOKEN_TTL_SECS` | `2592000` | OAuth refresh-token lifetime |
| `SYNAPSE_MCP_AUTH_CODE_TTL_SECS` | `300` | OAuth authorization-code lifetime |
| `SYNAPSE_MCP_AUTH_REGISTER_REQUESTS_PER_MINUTE` | `10` | Dynamic-registration rate limit |
| `SYNAPSE_MCP_AUTH_AUTHORIZE_REQUESTS_PER_MINUTE` | `60` | Authorization endpoint rate limit |
| `SYNAPSE_MCP_AUTH_DISABLE_STATIC_TOKEN_WITH_OAUTH` | `true` | Reject the static token while OAuth is active |

## Host Discovery

| Variable | Purpose |
|---|---|
| `SYNAPSE_HOSTS_CONFIG` | Inline host topology as a JSON array; highest priority |
| `SYNAPSE_CONFIG_FILE` | Path to a hosts config file; used when inline hosts are unset |

When neither variable is set, Synapse2 falls back to `~/.ssh/config` discovery.
Per-host `exec_allowlist` entries extend the built-in safe read command list.

## Auth Policy

| State | Condition | Behavior |
|---|---|---|
| `LoopbackDev` | loopback bind or loopback no-auth | no auth, no scopes |
| `Mounted` bearer | non-loopback with `SYNAPSE_MCP_TOKEN` | bearer auth and scope checks |
| `Mounted` OAuth | `SYNAPSE_MCP_AUTH_MODE=oauth` | OAuth/JWT auth and scope checks |
