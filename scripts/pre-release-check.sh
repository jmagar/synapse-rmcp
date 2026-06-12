#!/usr/bin/env bash
# Run the template release-readiness gate.
set -euo pipefail

RUN_VERIFY=true
RUN_BUILD_PLUGIN=true
RUN_MCPORTER=false

usage() {
  cat <<'EOF'
Usage: scripts/pre-release-check.sh [OPTIONS]

Options:
  --skip-verify        Skip `just verify`.
  --skip-build-plugin  Skip `just build-plugin`.
  --mcporter           Also run `just test-mcporter` (requires running server).
  -h, --help           Show this help.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --skip-verify) RUN_VERIFY=false; shift ;;
    --skip-build-plugin) RUN_BUILD_PLUGIN=false; shift ;;
    --mcporter) RUN_MCPORTER=true; shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "unknown argument: $1" >&2; usage >&2; exit 2 ;;
  esac
done

PASS=0
FAIL=0
FAILED_CHECKS=()

run_check() {
  local label="$1"
  shift
  printf '\n==> %s\n' "$label"
  if "$@"; then
    printf 'PASS %s\n' "$label"
    PASS=$((PASS + 1))
  else
    printf 'FAIL %s\n' "$label" >&2
    FAILED_CHECKS+=("$label")
    FAIL=$((FAIL + 1))
  fi
}

run_packaging_identity_check() {
  local failed=0

  grep -q 'BINARY_NAME: synapse' .github/workflows/release.yml || {
    printf 'release workflow must package the synapse binary\n' >&2
    failed=1
  }
  grep -q 'IMAGE_NAME: ghcr.io/jmagar/synapse2' .github/workflows/docker-publish.yml || {
    printf 'Docker workflow must publish ghcr.io/jmagar/synapse2\n' >&2
    failed=1
  }
  grep -q 'image-ref: ghcr.io/jmagar/synapse2:latest' .github/workflows/docker-publish.yml || {
    printf 'Trivy scan must target ghcr.io/jmagar/synapse2:latest\n' >&2
    failed=1
  }

  if grep -E 'your-org/example-mcp|BINARY_NAME="example"|EXAMPLE_MCP_|localhost:3000' install.sh; then
    printf 'install.sh still contains template identity values\n' >&2
    failed=1
  fi

  return "$failed"
}

run_check "packaging identity" run_packaging_identity_check
run_check "PATTERNS.md contracts" cargo xtask patterns
run_check "plugin layout" just validate-plugin
run_check "schema docs" python3 scripts/check-schema-docs.py --check
run_check "OpenAPI docs" python3 scripts/check-openapi.py --check
run_check "scaffold intent contract" python3 scripts/check-scaffold-intent-contract.py
run_check "template feature smoke" bash scripts/test-template-features.sh
run_check "version sync" bash scripts/check-version-sync.sh
run_check "blob size" python3 scripts/check-blob-size.py
run_check "ascii hygiene" just ascii-check

if [[ "$RUN_VERIFY" == true ]]; then
  run_check "quality gate" just verify
fi

if [[ "$RUN_BUILD_PLUGIN" == true ]]; then
  run_check "plugin binary build" just build-plugin
fi

if [[ "$RUN_MCPORTER" == true ]]; then
  run_check "mcporter integration" just test-mcporter
fi

printf '\n== Results ==\n'
printf 'Passed: %d\nFailed: %d\n' "$PASS" "$FAIL"
if (( FAIL > 0 )); then
  printf 'Failed checks:\n' >&2
  printf '  - %s\n' "${FAILED_CHECKS[@]}" >&2
  exit 1
fi

printf 'Release gate passed.\n'
