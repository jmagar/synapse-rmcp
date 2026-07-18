#!/usr/bin/env python3
"""Fail unless cargo-deny's only yanked crate is the time-bounded spin exception."""

import datetime as dt
import json
import subprocess
import sys

ALLOWED = {("spin", "0.9.8")}
EXPIRES = dt.date(2026, 10, 1)

result = subprocess.run(
    ["cargo", "deny", "--format", "json", "check", "advisories"],
    check=False,
    text=True,
    stdout=subprocess.PIPE,
    stderr=subprocess.STDOUT,
)
if result.returncode:
    sys.stdout.write(result.stdout)
    raise SystemExit(result.returncode)

yanked = set()
for line in result.stdout.splitlines():
    try:
        event = json.loads(line)
    except json.JSONDecodeError:
        continue
    fields = event.get("fields", {})
    if fields.get("code") != "yanked":
        continue
    graphs = fields.get("graphs", [])
    package = graphs[0].get("Krate", {}) if graphs else {}
    yanked.add((package.get("name"), package.get("version")))

if yanked != ALLOWED:
    print(f"unexpected yanked dependency set: {sorted(yanked)}; allowed: {sorted(ALLOWED)}")
    raise SystemExit(1)
if dt.date.today() >= EXPIRES:
    print(f"spin exception expired on {EXPIRES}; migrate lab-auth before renewing")
    raise SystemExit(1)
print(f"accepted time-bounded yanked exception: spin 0.9.8 through {EXPIRES}")
