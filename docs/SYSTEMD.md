---
title: "systemd Deployment"
doc_type: "guide"
status: "active"
owner: "synapse2"
audience:
  - "contributors"
  - "agents"
scope: "synapse2"
source_of_truth: false
last_reviewed: "2026-06-12"
---

# systemd

Synapse2 can run as a user-level systemd service when the release binary is
installed as `synapse`.

## Install the binary

```bash
cargo build --release
install -m 755 target/release/synapse ~/.local/bin/synapse
```

Or use the Justfile recipe:

```bash
just install-local
```

Verify the installed binary:

```bash
synapse --version
synapse doctor
```

## Unit file

Create `~/.config/systemd/user/synapse2.service`:

```ini
[Unit]
Description=Synapse2 MCP server
After=network.target

[Service]
Type=simple
ExecStart=%h/.local/bin/synapse serve mcp
Restart=on-failure
RestartSec=5
EnvironmentFile=%h/.synapse2/.env

[Install]
WantedBy=default.target
```

Key points:

- Use `EnvironmentFile=%h/.synapse2/.env`; never hardcode tokens in unit files.
- `%h` expands to the user home directory.
- `serve mcp` is the canonical Streamable HTTP mode.
- For bearer auth, set `SYNAPSE_MCP_TOKEN` in the environment file.

## Restart flow

```bash
systemctl --user daemon-reload
systemctl --user enable --now synapse2.service
systemctl --user restart synapse2.service
systemctl --user status synapse2.service
```

## Runtime verification

`just runtime-current` detects stale running processes. The checker compares the
running process executable with the expected release binary:

```bash
scripts/check-runtime-current.sh --mode systemd --expected-binary target/release/synapse
just runtime-current
```

If hashes differ, install the new binary and restart the unit.

## Logging

With systemd, logs go to the journal:

```bash
journalctl --user -u synapse2.service -f
journalctl --user -u synapse2.service --since "1h ago"
```

The binary also writes structured JSON logs to `~/.synapse2/logs/synapse.log`
when file logging is enabled by the runtime configuration.

## Doctor pre-flight

Run `synapse doctor` before starting the unit to validate the environment:

```bash
synapse doctor
```

Exit code 0 means ready to start. Exit code 1 means one or more issues were
found.

## Environment

Prefer an `EnvironmentFile` that points at the appdata `.env`. See
`docs/ENV.md` for variable meanings and `docs/AUTH.md` for auth policy details.
