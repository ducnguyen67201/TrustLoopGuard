#!/usr/bin/env bash
# Mechanical enforcement of two apps/web boundary rules:
#
#   1. Browser-bundled code in apps/web MUST NOT talk to tl-server or
#      any third-party API directly. It can only call our own Next API
#      routes under /api/*. The tl-server URL stays server-side.
#
#   2. LLM provider SDKs (openai, anthropic, …) MUST NOT appear ANYWHERE
#      in apps/web — not even in app/api/** server routes. Those calls
#      belong in tl-server (Rust), exposed as a typed endpoint and
#      reached from web via the @trustloopguard/sdk Client. This keeps
#      provider integrations, secrets, rate limits, and observability
#      in one place and gives the CLI, Python SDK, and Rust SDK the
#      same feature for free.
#
# Server-side paths inside apps/web (where the SDK Client and absolute-
# URL fetch are still allowed):
#
#   - apps/web/app/api/**       (Next API route handlers — server-side)
#   - apps/web/lib/server/**    (server-only helpers)
#   - apps/web/auth.ts          (NextAuth wiring)
#   - apps/web/env.ts           (env schema, no logic)
#
# Bypassing this lint defeats the whole point of the proxy layer and
# the SDK-driven contract: it isolates secrets, gives us one place to
# add auth / rate limits / observability, and keeps every SDK consumer
# at parity with the web app.

set -euo pipefail

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "${ROOT}"

WEB_DIR="apps/web"

if [[ ! -d "${WEB_DIR}" ]]; then
  echo "skip: ${WEB_DIR} does not exist"
  exit 0
fi

# Files that are allowed to use the SDK Client, provider SDKs, or
# absolute-URL fetch — they run on the Node server only.
is_server_side() {
  local path="$1"
  case "${path}" in
    ${WEB_DIR}/app/api/*) return 0 ;;
    ${WEB_DIR}/lib/server/*) return 0 ;;
    ${WEB_DIR}/auth.ts) return 0 ;;
    ${WEB_DIR}/env.ts) return 0 ;;
    *) return 1 ;;
  esac
}

violations=()

PROVIDER_SDK_PATTERN="^(import|export)[^'\"]*from[[:space:]]+['\"](openai|@anthropic-ai/sdk|@google/generative-ai|cohere-ai|@mistralai/mistralai)['\"]"

scan_provider_sdks_anywhere() {
  local file="$1"
  # Rule 2: provider SDKs are forbidden ANYWHERE in apps/web — these
  # calls live in tl-server (Rust). No exemption for app/api/**.
  if grep -nE "${PROVIDER_SDK_PATTERN}" "${file}" >/dev/null 2>&1; then
    while IFS= read -r line; do
      violations+=("${file}: LLM provider SDK imported — move this call into tl-server (Rust) and proxy via @trustloopguard/sdk → ${line}")
    done < <(grep -nE "${PROVIDER_SDK_PATTERN}" "${file}")
  fi
}

scan_browser_only_rules() {
  local file="$1"
  if is_server_side "${file}"; then
    return
  fi

  # Rule 1a: SDK Client value imports — server-only. Type-only imports
  # are fine because they get erased.
  if grep -nE "^[[:space:]]*import[[:space:]]+(\{[^}]*\bClient\b[^}]*\}|Client)[[:space:]]+from[[:space:]]+['\"]@trustloopguard/sdk['\"]" \
      "${file}" >/dev/null 2>&1; then
    while IFS= read -r line; do
      case "${line}" in
        *"import type"*) continue ;;
      esac
      violations+=("${file}: @trustloopguard/sdk Client imported in browser-bundled code → ${line}")
    done < <(grep -nE "^[[:space:]]*import[[:space:]]+(\{[^}]*\bClient\b[^}]*\}|Client)[[:space:]]+from[[:space:]]+['\"]@trustloopguard/sdk['\"]" "${file}")
  fi

  # Rule 1b: Absolute-URL fetch — browser code must call same-origin /api/*.
  if grep -nE "fetch\([[:space:]]*[\"'\`]https?://" "${file}" >/dev/null 2>&1; then
    while IFS= read -r line; do
      violations+=("${file}: fetch() with absolute URL in browser-bundled code → ${line}")
    done < <(grep -nE "fetch\([[:space:]]*[\"'\`]https?://" "${file}")
  fi
}

scan_file() {
  local file="$1"
  scan_provider_sdks_anywhere "${file}"
  scan_browser_only_rules "${file}"
}

while IFS= read -r -d '' file; do
  scan_file "${file}"
done < <(find "${WEB_DIR}" -type f \( -name "*.ts" -o -name "*.tsx" \) \
  -not -path "*/node_modules/*" \
  -not -path "*/.next/*" \
  -print0)

if (( ${#violations[@]} > 0 )); then
  echo "boundary lint failed: ${#violations[@]} violation(s)" >&2
  echo "" >&2
  echo "apps/web rules:" >&2
  echo "  - Browser-bundled code may only call same-origin /api/* routes." >&2
  echo "    Move calls behind a Next route in apps/web/app/api/, or a" >&2
  echo "    server-only helper in apps/web/lib/server/." >&2
  echo "  - LLM provider SDKs (openai, anthropic, …) are forbidden anywhere" >&2
  echo "    in apps/web — those calls live in tl-server (Rust) and reach" >&2
  echo "    web via @trustloopguard/sdk." >&2
  echo "" >&2
  for v in "${violations[@]}"; do
    printf '  %s\n' "${v}" >&2
  done
  exit 1
fi

echo "apps/web boundary lint: OK"
