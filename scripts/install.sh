#!/usr/bin/env bash
set -euo pipefail

# Compatibility entrypoint retained for callers of scripts/install.sh.
# The repository-root installer is the single implementation and contract.
script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
exec "${script_dir}/../install.sh" "$@"
