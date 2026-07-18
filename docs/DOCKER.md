---
title: "Docker"
doc_type: "guide"
status: "active"
owner: "synapse2"
audience:
  - "contributors"
  - "agents"
scope: "synapse2"
source_of_truth: false
upstream_refs:
  - "config/Dockerfile"
  - "docker-compose.yml"
  - "docker-compose.prod.yml"
last_reviewed: "2026-06-12"
---

# Docker

Synapse2 Docker support lives in `config/Dockerfile`, `docker-compose.yml`, and
`docker-compose.prod.yml`. The image builds the Next.js static web export, embeds
it in the Rust binary, and runs the `synapse` binary on port `40080`.

## Common commands

```bash
just docker-build      # build image
just docker-up         # start compose stack
just docker-down       # stop stack
just docker-rebuild    # rebuild image and recreate container
just docker-logs       # follow logs
just runtime-current   # compare running service with the local binary
```

## Image layout

`config/Dockerfile` uses three stages:

| Stage | Purpose |
|---|---|
| `web` | Build `apps/web/out` with pnpm. |
| `builder` | Compile package `synapse2` and copy `target/release/synapse` to `/usr/local/bin/synapse`. |
| `runtime` | Minimal Debian runtime with `curl`, `gosu`, and `openssh-client`. Docker access uses Bollard over the mounted socket; the Docker CLI is not installed. |

The runtime image exposes `40080/tcp`, healthchecks `http://localhost:40080/health`,
and starts with:

```dockerfile
ENTRYPOINT ["/entrypoint.sh"]
CMD ["serve", "mcp"]
```

`entrypoint.sh` fixes `/data` permissions, hardens `config.toml`, `.env`,
`auth-jwt.pem`, and `auth.db`, then execs `/usr/local/bin/synapse` as UID 1000.

## Compose service

The local compose file extends the production service and builds `synapse2:dev`.
The production service uses:

```yaml
services:
  synapse2:
    image: synapse2:${SYNAPSE2_VERSION:-latest}
    container_name: synapse2
    env_file:
      - path: .env
        required: false
    environment:
      SYNAPSE_MCP_HOST: 0.0.0.0
      SYNAPSE_MCP_PORT: "40080"
      SYNAPSE_MCP_TOKEN: "${SYNAPSE_MCP_TOKEN:-}"
    ports:
      - "${SYNAPSE_MCP_BIND_HOST:-127.0.0.1}:${SYNAPSE_MCP_HOST_PORT:-40080}:40080/tcp"
    volumes:
      - ${HOME}/.synapse2:/data
      - /var/run/docker.sock:/var/run/docker.sock
      - ${HOME}/.ssh:/home/synapse/.ssh:ro
    group_add:
      - "${DOCKER_GID:?set DOCKER_GID from: getent group docker | cut -d: -f3}"
```

Key requirements:

- Create the external Docker network first when needed: `docker network create mcp`.
- Set `DOCKER_GID` when mounting `/var/run/docker.sock`; otherwise `flux` Docker
  actions will not reach the daemon.
- Set `SYNAPSE_MCP_TOKEN` for bearer deployments, or configure OAuth explicitly.
- For Labby or another trusted gateway, `SYNAPSE_NOAUTH=true` delegates auth/authz
  to that gateway. Keep the container port isolated from every other peer.
- `SYNAPSE_MCP_HOST=0.0.0.0` is the container-internal bind. Keep the host
  publish address at `SYNAPSE_MCP_BIND_HOST=127.0.0.1` unless an authenticated
  reverse proxy or firewall is the explicit network boundary.
- Mount `~/.ssh` read-only at `/home/synapse/.ssh` so `scout` host discovery can
  read the operator SSH config.

## Appdata convention

Local binary and Docker share the same logical data directory:

| Deployment | Data directory |
|---|---|
| Local binary | `~/.synapse2/` |
| Docker | `/data/` inside container, mounted from `~/.synapse2/` on host |
| Plugin | `$CLAUDE_PLUGIN_DATA`, or `SYNAPSE_HOME` when explicitly set |

`config.toml`, `.env`, OAuth state, and JWT signing keys belong in this data
directory. Do not bake secrets into the image.

## Health and auth

- `/health` is unauthenticated and used by Docker healthchecks.
- `/mcp` and `/v1/synapse2` require auth outside loopback unless
  `SYNAPSE_NOAUTH=true` explicitly delegates auth/authz to an isolated trusted gateway.
- Recreate the container after editing `.env`:

```bash
docker compose up -d --force-recreate
```

Use `just auth-smoke` for a bearer-auth smoke test against a running server.

## Versioned rollout and rollback

Production deployments should pin `SYNAPSE2_VERSION` to a release tag such as
`0.5.4` or an immutable commit tag such as `sha-<full-commit>`, rather than rely
on `latest`:

```bash
SYNAPSE2_VERSION=0.5.4 docker compose -f docker-compose.prod.yml pull
SYNAPSE2_VERSION=0.5.4 docker compose -f docker-compose.prod.yml up -d
curl -fsS http://127.0.0.1:40080/ready
```

Record the previous value before rollout. If readiness or the authenticated
smoke test fails, restore it and recreate the service:

```bash
SYNAPSE2_VERSION=<previous-tag> docker compose -f docker-compose.prod.yml pull
SYNAPSE2_VERSION=<previous-tag> docker compose -f docker-compose.prod.yml up -d --force-recreate
```

## Build artifacts

`just build-plugin` copies the release binary to `bin/synapse` and
`plugins/synapse2/bin/synapse`. The runtime freshness check compares running
processes against `target/release/synapse`.
