#!/usr/bin/env python3
"""Reject stale template identities, default ports, and split action registries."""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def read(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


def main() -> int:
    failures: list[str] = []
    forbidden = {
        "apps/web/components/api/action-card.tsx": ["localhost:3100", "localhost:3000"],
        "plugins/README.md": ["plugins/example", "skills/example", "example_api_", "localhost:3000"],
        "scripts/README.md": [
            "EXAMPLE_MCP_",
            "/v1/example",
            "target/release/example",
            "localhost:3100",
            "plugins/example",
            "ACTION_SPECS",
        ],
        "docs/WEB.md": ["sessionStorage"],
    }
    for path, needles in forbidden.items():
        text = read(path)
        for needle in needles:
            if needle in text:
                failures.append(f"{path} contains stale identity/contract value {needle!r}")

    mcporter = json.loads(read("config/mcporter.json"))
    expected = {"url": "http://localhost:40080/mcp", "transport": "http"}
    if mcporter.get("mcpServers") != {"synapse2": expected}:
        failures.append("config/mcporter.json must define only synapse2 at localhost:40080/mcp")

    operations = read("src/actions/operations.rs")
    if len(re.findall(r"^\s*operation!\(", operations, re.MULTILINE)) != 59:
        failures.append("OPERATION_SPECS must contain the 59 production operations")
    actions = read("src/actions.rs")
    for stale_registry in ["ACTION_SPECS", "MCP_OPERATION_NAMES"]:
        if stale_registry in actions:
            failures.append(f"src/actions.rs reintroduced split registry {stale_registry}")

    if failures:
        for failure in failures:
            print(f"FAIL: {failure}", file=sys.stderr)
        return 1
    print("identity and operation registry contracts are current")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
