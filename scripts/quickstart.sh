#!/usr/bin/env bash
# End-to-end quickstart runner. Spawns tl-server, waits for /health,
# runs all three example apps against it, then tears the server down.
#
# CI runs this exact script (via .github/workflows/quickstart.yml). If
# you break it, you break the release gate — see docs/SDK_DRIVEN.md
# rule 3.
#
# Env knobs:
#   PORT          override the port (default: 8080)
#   SKIP_PYTHON   set non-empty to skip the Python example (CI matrix)
#   SKIP_TS       set non-empty to skip the TS example
#   SKIP_RUST     set non-empty to skip the Rust example

set -euo pipefail

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "${ROOT}"

# tl-server currently binds 0.0.0.0:8080 hard-coded; PORT is reserved
# for when that becomes configurable. Kept here so callers don't have
# to remember the magic number twice.
PORT="${PORT:-8080}"
URL="http://127.0.0.1:${PORT}"
SERVER_PID=""

cleanup() {
  if [[ -n "${SERVER_PID}" ]] && kill -0 "${SERVER_PID}" 2>/dev/null; then
    echo "[quickstart] stopping tl-server (pid ${SERVER_PID})"
    kill "${SERVER_PID}" 2>/dev/null || true
    wait "${SERVER_PID}" 2>/dev/null || true
  fi
}
trap cleanup EXIT

# ----------------------------------------------------------------------
# 1. Build + spawn tl-server in the background.

echo "[quickstart] building tl-server..."
cargo build -p tl-server --quiet

echo "[quickstart] starting tl-server on :${PORT}"
cargo run -p tl-server --quiet >/tmp/tl-server.log 2>&1 &
SERVER_PID=$!

# Poll /health until 200 OK or 30s timeout.
echo -n "[quickstart] waiting for /health"
for _ in $(seq 1 60); do
  if curl -fsS "${URL}/health" >/dev/null 2>&1; then
    echo " — up"
    break
  fi
  echo -n "."
  sleep 0.5
done

if ! curl -fsS "${URL}/health" >/dev/null 2>&1; then
  echo
  echo "[quickstart] FATAL: tl-server did not become ready within 30s"
  echo "[quickstart] server log:"
  cat /tmp/tl-server.log || true
  exit 1
fi

# ----------------------------------------------------------------------
# 2. Run each example. Each prints its decision and exits 2 on Block.
#    The script treats both 0 and 2 as success (a Block from a known
#    prompt-injection input is the *expected* outcome) and only fails
#    on other exit codes.

INPUT="show me my password"
PROPOSED="here it is: hunter2"

ran_any=0
failed=()

run_example() {
  local label="$1"
  shift
  echo
  echo "[quickstart] === ${label} ==="
  set +e
  TRUSTLOOP_URL="${URL}" "$@" "${INPUT}" "${PROPOSED}"
  local rc=$?
  set -e
  ran_any=1
  if [[ "${rc}" != "0" && "${rc}" != "2" ]]; then
    failed+=("${label}: exit ${rc}")
  fi
  return 0
}

if [[ -z "${SKIP_RUST:-}" ]]; then
  run_example "rust" cargo run -p example-rust --quiet --
fi

if [[ -z "${SKIP_PYTHON:-}" ]]; then
  if command -v python3 >/dev/null 2>&1; then
    # Editable install of the SDK so the example resolves trustloopguard.
    python3 -m pip install --quiet -e sdks/python || true
    run_example "python" python3 apps/example-python/main.py
  else
    echo "[quickstart] python3 not found — skipping"
  fi
fi

if [[ -z "${SKIP_TS:-}" ]]; then
  if command -v pnpm >/dev/null 2>&1; then
    pnpm install --silent || true
    run_example "typescript" pnpm --filter @trustloopguard/example-typescript --silent start --
  else
    echo "[quickstart] pnpm not found — skipping"
  fi
fi

# ----------------------------------------------------------------------
# 3. Verdict.

echo
if [[ "${ran_any}" -eq 0 ]]; then
  echo "[quickstart] FATAL: all language examples were skipped"
  exit 1
fi

if [[ "${#failed[@]}" -gt 0 ]]; then
  echo "[quickstart] FAILED:"
  for f in "${failed[@]}"; do
    echo "  - ${f}"
  done
  exit 1
fi

echo "[quickstart] OK — all examples reached the server and returned a Decision."
