#!/usr/bin/env bash
# Hermetic live HTTP JSON-RPC smoke. The caller owns server/SSH fixture setup.
set -euo pipefail

base_url="${SYNAPSE_TEST_BASE_URL:-http://127.0.0.1:40080}"
response="$(mktemp)"
trap 'rm -f "${response}"' EXIT

for _attempt in {1..60}; do
  if curl -fsS "${base_url}/ready" >/dev/null; then break; fi
  sleep 1
done
curl -fsS "${base_url}/ready" | python3 -c 'import json,sys; assert json.load(sys.stdin)["status"] == "ready"'

call() {
  local tool="$1" arguments="$2"
  python3 - "${tool}" "${arguments}" <<'PY' | curl -fsS \
    -H 'Content-Type: application/json' -H 'Accept: application/json, text/event-stream' \
    --data-binary @- "${base_url}/mcp" >"${response}"
import json,sys
print(json.dumps({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":sys.argv[1],"arguments":json.loads(sys.argv[2])}}))
PY
}

call scout '{"action":"nodes"}'
python3 - "${response}" <<'PY'
import json,sys
payload=json.load(open(sys.argv[1]))
assert "error" not in payload, payload
assert payload["result"]["content"], payload
PY

call scout '{"action":"peek","host":"ci-ssh","path":"/tmp/synapse-ci-fixture","lines":5}'
python3 - "${response}" <<'PY'
import json,sys
payload=json.load(open(sys.argv[1]))
assert "error" not in payload, payload
content=payload["result"]["content"][0]
text=json.dumps(content)
assert "synapse-live-ssh-ok" in text, payload
PY

echo "live JSON-RPC and SSH smoke passed"
