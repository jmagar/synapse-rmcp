# synapse2 Quickstart

This guide gets a local checkout running with loopback-only no-auth MCP HTTP,
then exercises the current CLI and REST action surfaces.

## 1. Start local dev mode

```bash
export SYNAPSE_MCP_HOST=127.0.0.1
export SYNAPSE_MCP_PORT=40080
export SYNAPSE_MCP_NO_AUTH=true
export RUST_LOG=info

cargo run --locked -- serve
```

In another shell:

```bash
curl -s http://127.0.0.1:40080/health | jq .
curl -s http://127.0.0.1:40080/status | jq .
```

Loopback no-auth is only for local development. Non-loopback binds require a
bearer token or OAuth, including when deployed behind a gateway.

## 2. Try the CLI

```bash
cargo run --locked -- flux docker info --host local
cargo run --locked -- flux container list --host local
cargo run --locked -- scout nodes
cargo run --locked -- scout exec --host local --command hostname
```

Synapse2 discovers SSH hosts from `~/.ssh/config` and always appends the built-in
`local` host. Docker actions against `local` use `/var/run/docker.sock`.

## 3. Try the REST action endpoint

```bash
curl -s http://127.0.0.1:40080/v1/synapse2 \
  -H "Content-Type: application/json" \
  -d '{"action":"flux.docker.info","params":{"host":"local"}}' | jq .
```

REST is a thin compatibility surface. MCP and CLI are the primary supported
interfaces.

## 4. Try MCP through mcporter

```bash
scripts/mcporter/test-mcp.sh
```

The script starts the server, lists MCP tools/resources/prompts, and verifies
the `flux` and `scout` tool contracts against the live HTTP endpoint.

## 5. Verify before pushing

```bash
cargo fmt --check
cargo test --locked
cargo clippy --locked -- -D warnings
cargo build --locked --release
```
