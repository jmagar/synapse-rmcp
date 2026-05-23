# synapse2 Configuration

## MCP

| Variable | Default | Purpose |
|---|---|---|
| `SYNAPSE2_MCP_HOST` | `0.0.0.0` | HTTP bind host |
| `SYNAPSE2_MCP_PORT` | `3100` | HTTP bind port |
| `SYNAPSE2_MCP_TOKEN` | unset | Static bearer token |
| `SYNAPSE2_MCP_NO_AUTH` | false | Disable auth on loopback only |
| `SYNAPSE2_NOAUTH` | false | Explicit trusted gateway mode |
| `SYNAPSE2_MCP_ALLOWED_HOSTS` | unset | Extra Host header values |
| `SYNAPSE2_MCP_ALLOWED_ORIGINS` | unset | Extra CORS origins |
| `SYNAPSE2_MCP_AUTH_MODE` | `bearer` | `bearer` or `oauth` |

## Scout Guardrails

| Variable | Purpose |
|---|---|
| `SYNAPSE2_SCOUT_ALLOWED_COMMANDS` | Comma-separated command allowlist |
| `SYNAPSE2_SCOUT_DENIED_PATTERNS` | Comma-separated rejected path/shell patterns |

Defaults are intentionally conservative. Extend the allowlist only for commands that are safe to expose to agents.

## Auth Policy

| State | Condition | Behavior |
|---|---|---|
| `LoopbackDev` | loopback bind or loopback no-auth | no auth, no scopes |
| `TrustedGatewayUnscoped` | `SYNAPSE2_NOAUTH=true` behind a trusted gateway | no local auth or scopes |
| `Mounted` bearer | non-loopback with `SYNAPSE2_MCP_TOKEN` | bearer auth and scope checks |
| `Mounted` OAuth | `SYNAPSE2_MCP_AUTH_MODE=oauth` | OAuth/JWT auth and scope checks |
