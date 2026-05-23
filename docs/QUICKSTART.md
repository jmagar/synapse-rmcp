# synapse2 Quickstart

## 1. Configure local dev

```bash
export SYNAPSE2_MCP_HOST=127.0.0.1
export SYNAPSE2_MCP_PORT=3100
export SYNAPSE2_MCP_NO_AUTH=true
export SYNAPSE2_SCOUT_ALLOWED_COMMANDS=docker,hostname,uptime,whoami
```

## 2. Try the CLI

```bash
cargo run -- flux docker-info
cargo run -- scout nodes
cargo run -- scout exec --command hostname
```

## 3. Start HTTP MCP

```bash
cargo run -- serve
```

```bash
curl -s http://127.0.0.1:3100/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"flux","arguments":{"action":"docker_info"}}}'
```

## 4. Verify

```bash
cargo fmt --check
cargo test
cargo clippy -- -D warnings
cargo build --release
```
