#!/usr/bin/env bash
# Stop, rebuild, and restart the synapse2 service.
# Must be run from the repository root.
# Supports systemd user units and Docker Compose.
set -euo pipefail

UNIT="${SYNAPSE_MCP_SYSTEMD_UNIT:-synapse2.service}"
SERVICE="${SYNAPSE_MCP_DOCKER_SERVICE:-synapse2}"
BINARY="${SYNAPSE_MCP_BINARY:-synapse}"

echo "==> Repair target: unit=${UNIT} container=${SERVICE} binary=${BINARY}"
echo "==> Stopping synapse2..."
if systemctl --user is-active --quiet "${UNIT}" 2>/dev/null; then
    systemctl --user stop "${UNIT}"
    echo "    stopped systemd unit"
elif docker ps --filter "name=^/${SERVICE}$" --quiet 2>/dev/null | grep -q .; then
    docker stop "${SERVICE}" >/dev/null
    echo "    stopped docker container"
else
    echo "    no running instance found"
fi

echo "==> Rebuilding release binary..."
cargo build --release

echo "==> Restarting..."
if systemctl --user list-unit-files "${UNIT}" 2>/dev/null | grep -qF "${UNIT}"; then
    mkdir -p "${HOME}/.local/bin"
    install -m 755 "target/release/${BINARY}" "${HOME}/.local/bin/${BINARY}"
    systemctl --user start "${UNIT}"
    echo "    started systemd unit"
elif [ -f docker-compose.yml ]; then
    docker compose build
    docker compose up -d --force-recreate
    echo "    started docker compose service"
else
    echo "    no service manager detected; binary at target/release/${BINARY}"
fi

echo "==> Done"
