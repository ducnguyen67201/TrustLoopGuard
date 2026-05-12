#!/usr/bin/env bash
# Drives `oha` against a running tl-server. Pass a scenario name from
# scenarios/ as the first argument; any extra args go straight to oha.
#
# Examples:
#   ./loadtest/run.sh allow
#   ./loadtest/run.sh pii_block -n 5000 -c 100
#   ./loadtest/run.sh cache_hit -z 30s   # 30-second duration mode

set -euo pipefail

if ! command -v oha >/dev/null 2>&1; then
  echo "error: oha not installed. cargo install oha (or brew install oha)" >&2
  exit 1
fi

scenario="${1:-allow}"
shift || true

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
scenario_path="$script_dir/scenarios/${scenario}.json"

if [[ ! -f "$scenario_path" ]]; then
  echo "error: unknown scenario '$scenario'. available:" >&2
  ls "$script_dir/scenarios" | sed 's/\.json$//' | sed 's/^/  /' >&2
  exit 1
fi

SERVER_URL="${TL_SERVER_URL:-http://127.0.0.1:8080}"
API_KEY="${TL_API_KEY:-}"

headers=(-H 'content-type: application/json')
if [[ -n "$API_KEY" ]]; then
  headers+=(-H "authorization: Bearer $API_KEY")
fi

# Defaults: 1000 requests, 50 concurrent. Override via positional args.
default_args=(-n 1000 -c 50)

echo "▶ scenario : $scenario"
echo "▶ server   : $SERVER_URL"
echo "▶ body     : $scenario_path"
echo

exec oha "${default_args[@]}" "$@" \
  -m POST \
  "${headers[@]}" \
  -D "$scenario_path" \
  "$SERVER_URL/v1/check"
