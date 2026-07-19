---
title: "MCP Registry Publishing Guide"
doc_type: "guide"
status: "active"
owner: "synapse2"
audience:
  - "contributors"
  - "agents"
scope: "service"
source_of_truth: false
upstream_refs:
  - "server.json"
last_reviewed: "2026-06-13"
---

# MCP Registry Publishing Guide

This guide explains how to publish Synapse2 to the
[official MCP registry](https://modelcontextprotocol.io/registry/quickstart)
using the `server.json` manifest at the repo root.

## Current manifest

`server.json` is already adapted for this repo:

| Field | Current value |
|---|---|
| `name` | `tv.tootie/synapse2` |
| `repository.url` | `https://github.com/jmagar/synapse-rmcp` |
| `packages[0].identifier` | `ghcr.io/jmagar/synapse:<version>` |
| Hosted remote | Not declared; add `remotes` only when a public hosted `/mcp` endpoint exists |

## Prerequisites

- You own the domain used in the `name` field.
- The Docker image is published to a public container registry such as GHCR.
- The GitHub repo is public.

## Install mcp-publisher

```bash
curl -fsSL \
  "https://github.com/modelcontextprotocol/registry/releases/latest/download/mcp-publisher_linux_amd64.tar.gz" \
  | tar xz mcp-publisher
chmod +x mcp-publisher
```

For other platforms, check the
[releases page](https://github.com/modelcontextprotocol/registry/releases).

## Authenticate

### DNS-based namespace

```bash
./mcp-publisher login dns \
  --domain tv.tootie \
  --private-key "$MCP_PRIVATE_KEY"
```

The private key must correspond to a DNS TXT record published at `_mcp.tv.tootie`.
Use the registry docs for the exact TXT record format.

### GitHub namespace fallback

```bash
./mcp-publisher login github
```

GitHub OAuth grants the `github.com/<user-or-org>/` namespace. If the manifest is
renamed to a GitHub namespace, update `server.json` before publishing.

## Publish

```bash
./mcp-publisher publish
```

This reads `server.json` from the current directory and submits it to the
registry. On success, the server appears at:

```text
https://registry.modelcontextprotocol.io/servers/tv.tootie/synapse2
```

## Version Management

`server.json` should reflect the currently released version. The release flow
updates the top-level `version`, `packages[0].version`, and
`packages[0].identifier` image tag when publishing a version tag.

```bash
git tag v1.2.3
git push origin v1.2.3
```

## CI Snippet

When publishing from a release workflow, update the manifest version and image
tag before calling the publisher:

```yaml
- name: Set version in server.json
  run: |
    VERSION="${GITHUB_REF_NAME#v}"
    jq --arg v "$VERSION" \
       --arg img "ghcr.io/jmagar/synapse:${VERSION}" \
       '.version = $v | .packages[0].identifier = $img | .packages[0].version = $v' \
       server.json > server.tmp && mv server.tmp server.json

- name: Publish to MCP registry
  env:
    MCP_PRIVATE_KEY: ${{ secrets.MCP_PRIVATE_KEY }}
  run: |
    ./mcp-publisher login dns --domain tv.tootie --private-key "$MCP_PRIVATE_KEY"
    ./mcp-publisher publish
```

## Troubleshooting

### "Name not in your namespace"

Authenticate for the domain or GitHub user that prefixes the manifest `name`.
For `tv.tootie/synapse2`, use DNS auth for `tv.tootie`.

### "Invalid schema"

Run the JSON through the schema validator:

```bash
npx @modelcontextprotocol/registry-validator server.json
```

### "Image not found"

The `packages[0].identifier` OCI image must be publicly pullable before publish.
Push to GHCR first, then publish to the registry.
