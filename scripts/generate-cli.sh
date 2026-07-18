#!/usr/bin/env bash
# Generate a standalone CLI for this server via mcporter.
# Must be run from the repository root.
# Requires: running server on port 40080 and mcporter in PATH.
# Generated CLI embeds your token — do not commit or share.
set -euo pipefail
umask 077

if ! command -v mcporter >/dev/null 2>&1; then
    echo "error: mcporter not found. Install it first." >&2
    exit 1
fi

echo "Server must be running on port 40080 (run 'just dev' first)"
echo "Generated CLI embeds your token — do not commit or share"

mkdir -p dist dist/.cache

schema_json="$(mktemp)"
trap 'rm -f "$schema_json"' EXIT

curl_headers=(-H "Accept: application/json, text/event-stream")
mcporter_args=(generate-cli --command http://localhost:40080/mcp --name synapse-cli --output dist/synapse-cli)
mcporter_list_args=(list http://localhost:40080/mcp --schema)
if [[ -n "${SYNAPSE_MCP_TOKEN:-}" ]]; then
    curl_headers+=(-H "Authorization: Bearer ${SYNAPSE_MCP_TOKEN}")
    mcporter_args+=(--header "Authorization: Bearer ${SYNAPSE_MCP_TOKEN}")
    mcporter_list_args+=(--header "Authorization: Bearer ${SYNAPSE_MCP_TOKEN}")
fi

if ! timeout 10 mcporter "${mcporter_list_args[@]}" >"$schema_json"; then
    echo "error: failed to fetch tool schema from http://localhost:40080/mcp" >&2
    exit 1
fi

current_hash=$(sha256sum "$schema_json" | cut -d' ' -f1)

cache_file="dist/.cache/synapse-cli.schema_hash"
if [[ -f "$cache_file" ]] && [[ "$(cat "$cache_file")" == "$current_hash" ]] && [[ -f "dist/synapse-cli" ]]; then
    echo "SKIP: tool schema unchanged — use existing dist/synapse-cli"
    exit 0
fi

timeout 30 mcporter "${mcporter_args[@]}"
chmod 700 dist/synapse-cli
if ! git check-ignore -q dist/synapse-cli; then
    echo "warning: dist/synapse-cli is not ignored; generated CLI embeds secrets and must not be committed" >&2
fi

printf '%s' "$current_hash" > "$cache_file"
echo "Generated dist/synapse-cli (requires bun at runtime)"
