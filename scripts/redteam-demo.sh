#!/usr/bin/env bash
# Orchestrates the red-team-vs-guard demo from the TrustLoopGuard repo.
#
# TrustLoopGuard owns the platform (tl-server, web) and the target agents
# (demo/redteam-target). TrustLoopRed is the sibling attacker repo — used only
# to run the red-team engine (runner:app) and the CLI attack (run_demo.py).
#
# Backgrounds long-running services and tracks pids in .local/pids.
set -uo pipefail

TLG="$(cd "$(dirname "$0")/.." && pwd)"
TLR="${TLR:-$(cd "$TLG/../TrustLoopRed" 2>/dev/null && pwd)}"
RUN="$TLG/.local"
PIDS="$RUN/pids"
mkdir -p "$RUN"

TL_URL="http://127.0.0.1:8080"
WEB_URL="http://127.0.0.1:3000"
OLLAMA_URL="http://127.0.0.1:11434"
RAW_URL="http://127.0.0.1:9101"
GUARDED_URL="http://127.0.0.1:9102"
RUNNER_URL="http://127.0.0.1:8799"
MODEL="${DEMO_MODEL:-qwen2.5:7b}"
# Optional: route guarded checks + the policy to a workspace so decisions show in
# the dashboard. Leave empty to use the default workspace.
WORKSPACE_ID="${GUARDED_WORKSPACE_ID:-}"

code() { curl -so /dev/null -w '%{http_code}' "$1" 2>/dev/null; }
up()   { [ "$(code "$1")" != "000" ]; }

# NB: the function is NOT named `ollama` — that would shadow the binary and the
# `ollama list` / `ollama pull` calls below would recurse into the function
# (fork bomb). Call the binary by absolute path.
OLLAMA_BIN="$(command -v ollama || echo /opt/homebrew/opt/ollama/bin/ollama)"
start_ollama() {
  if curl -sf "$OLLAMA_URL/api/tags" >/dev/null 2>&1; then
    echo "ollama     already running"
  else
    echo "ollama     starting…"
    nohup "$OLLAMA_BIN" serve >"$RUN/ollama.log" 2>&1 &
    echo $! >>"$PIDS"
    until curl -sf "$OLLAMA_URL/api/tags" >/dev/null 2>&1; do sleep 1; done
  fi
  if ! "$OLLAMA_BIN" list 2>/dev/null | grep -q "$MODEL"; then
    echo "ollama     pulling $MODEL (one time)…"; "$OLLAMA_BIN" pull "$MODEL"
  fi
}

# be — the TrustLoopGuard gateway/API.
tl_server() {
  if curl -sf "$TL_URL/health" >/dev/null 2>&1; then
    echo "tl-server  already running"; return
  fi
  echo "tl-server  starting (first build can take a few minutes)…"
  ( cd "$TLG" && TL_LOG_FORMAT=pretty nohup doppler run -- cargo run -p tl-server >"$RUN/tl-server.log" 2>&1 & echo $! >>"$PIDS" )
  for _ in $(seq 1 300); do curl -sf "$TL_URL/health" >/dev/null 2>&1 && break; sleep 2; done
  if curl -sf "$TL_URL/health" >/dev/null 2>&1; then echo "tl-server  up"
  else echo "tl-server  FAILED — see $RUN/tl-server.log"; exit 1; fi
}

# fe — the dashboard web app.
web() {
  if up "$WEB_URL"; then
    echo "web        already running at $WEB_URL"
  else
    echo "web        starting at $WEB_URL (first compile ~20s)…"
    ( cd "$TLG/apps/web" && nohup doppler run -- pnpm dev >"$RUN/web.log" 2>&1 & echo $! >>"$PIDS" )
    echo "web        open $WEB_URL once it compiles (tail: $RUN/web.log)"
  fi
}

policy() {
  local file="$TLG/demo/redteam-target/policy.yaml" ok=0
  if [ -n "$WORKSPACE_ID" ]; then
    curl -sf -X POST "$TL_URL/v1/policies" -H 'content-type: application/yaml' \
      -H "x-tlg-workspace-id: $WORKSPACE_ID" --data-binary @"$file" >/dev/null 2>&1 && ok=1
  else
    curl -sf -X POST "$TL_URL/v1/policies" -H 'content-type: application/yaml' \
      --data-binary @"$file" >/dev/null 2>&1 && ok=1
  fi
  if [ "$ok" = 1 ]; then
    echo "policy     block-protected-disclosure loaded${WORKSPACE_ID:+ into $WORKSPACE_ID}"
  else
    echo "policy     load failed (is tl-server up?)"
  fi
}

agents() {
  for t in raw guarded; do
    port=$([ "$t" = raw ] && echo 9101 || echo 9102)
    if curl -sf "http://127.0.0.1:$port/health" >/dev/null 2>&1; then
      echo "$t agent  already running on :$port"
    else
      echo "$t agent  starting on :$port…"
      ( cd "$TLG" && TL_SERVER_URL="$TL_URL" GUARDED_WORKSPACE_ID="$WORKSPACE_ID" \
          nohup pnpm --filter @trustloopguard/demo "redteam:$t" >"$RUN/$t-agent.log" 2>&1 & echo $! >>"$PIDS" )
    fi
  done
  for _ in $(seq 1 30); do
    curl -sf "$RAW_URL/health" >/dev/null 2>&1 && curl -sf "$GUARDED_URL/health" >/dev/null 2>&1 && break
    sleep 1
  done
}

# The attacker lives in TrustLoopRed. We only invoke it.
runner() {
  [ -n "$TLR" ] && [ -x "$TLR/.venv/bin/uvicorn" ] || { echo "runner     SKIP — TrustLoopRed venv not found at $TLR/.venv"; return; }
  if curl -sf "$RUNNER_URL/health" >/dev/null 2>&1; then
    echo "runner     already running on :8799"
  else
    echo "runner     starting on :8799 (builtin attacks → 9101/9102)…"
    ( cd "$TLR" && ENGINE_MODE=builtin ARENA_RAW_URL="$RAW_URL" ARENA_GUARDED_URL="$GUARDED_URL" \
        nohup "$TLR/.venv/bin/uvicorn" runner:app --host 127.0.0.1 --port 8799 >"$RUN/runner.log" 2>&1 & echo $! >>"$PIDS" )
    for _ in $(seq 1 30); do curl -sf "$RUNNER_URL/health" >/dev/null 2>&1 && break; sleep 1; done
  fi
}

demo() {
  [ -n "$TLR" ] && [ -x "$TLR/.venv/bin/python" ] || { echo "attack SKIP — TrustLoopRed venv not found at $TLR/.venv"; exit 1; }
  ( cd "$TLR" && "$TLR/.venv/bin/python" demo/run_demo.py )
}

status() {
  printf "  %-10s %-30s %s\n" ollama    "$OLLAMA_URL/api/tags" "$(code "$OLLAMA_URL/api/tags")"
  printf "  %-10s %-30s %s\n" tl-server "$TL_URL/health"       "$(code "$TL_URL/health")"
  printf "  %-10s %-30s %s\n" raw-agent "$RAW_URL/health"      "$(code "$RAW_URL/health")"
  printf "  %-10s %-30s %s\n" guarded   "$GUARDED_URL/health"  "$(code "$GUARDED_URL/health")"
  printf "  %-10s %-30s %s\n" runner    "$RUNNER_URL/health"   "$(code "$RUNNER_URL/health")"
  printf "  %-10s %-30s %s\n" web       "$WEB_URL"             "$(code "$WEB_URL")"
}

down() {
  pkill -f "redteam-target/raw" 2>/dev/null && echo "stopped raw agent" || true
  pkill -f "redteam-target/guarded" 2>/dev/null && echo "stopped guarded agent" || true
  pkill -f "runner:app" 2>/dev/null && echo "stopped runner" || true
  pkill -f "cargo run -p tl-server" 2>/dev/null || true
  pkill -f "target/debug/tl-server" 2>/dev/null && echo "stopped tl-server" || true
  pkill -f "next dev" 2>/dev/null && echo "stopped web" || true
  echo "(ollama left running — stop with: pkill -f 'ollama serve')"
  rm -f "$PIDS"
}

case "${1:-}" in
  ollama)    start_ollama ;;
  tl-server) tl_server ;;
  web)       web ;;
  policy)    policy ;;
  agents)    agents ;;
  runner)    runner ;;
  demo)      demo ;;
  status)    status ;;
  down)      down ;;
  *) echo "usage: redteam-demo.sh {ollama|tl-server|web|policy|agents|runner|demo|status|down}"; exit 1 ;;
esac
