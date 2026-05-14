#!/usr/bin/env bash
# Mechanical enforcement of the apps/web boundary rule:
#
#   The browser-side code in apps/web MUST NOT talk to tl-server or any
#   third-party API directly. It can only call our own Next API routes
#   under /api/*. The tl-server URL, OpenAI API keys, and other secrets
#   stay server-side.
#
# Allowed server-side paths inside apps/web (where direct SDK / provider
# / absolute-URL fetch use IS permitted):
#
#   - apps/web/app/api/**       (Next API route handlers — server-side)
#   - apps/web/lib/server/**    (server-only helpers)
#   - apps/web/auth.ts          (NextAuth wiring)
#   - apps/web/env.ts           (env schema, no logic)
#
# Everything else is bundled into the browser and must hit only same-
# origin /api/* routes.
#
# Bypassing this lint defeats the whole point of the proxy layer: it
# isolates secrets, gives us one place to add auth / rate limits /
# observability, and stops the browser from depending on tl-server's URL.

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

scan_file() {
  local file="$1"
  if is_server_side "${file}"; then
    return
  fi

  # 1. Provider SDK imports — server-only.
  if grep -nE "^(import|export)[^'\"]*from[[:space:]]+['\"](openai|@anthropic-ai/sdk|@google/generative-ai|cohere-ai)['\"]" \
      "${file}" >/dev/null 2>&1; then
    while IFS= read -r line; do
      violations+=("${file}: provider SDK imported in browser-bundled code → ${line}")
    done < <(grep -nE "^(import|export)[^'\"]*from[[:space:]]+['\"](openai|@anthropic-ai/sdk|@google/generative-ai|cohere-ai)['\"]" "${file}")
  fi

  # 2. SDK Client class imports — server-only. Type-only imports are fine
  # because they get erased.
  #
  # Match a value import (no `import type`) that pulls a *value* binding
  # named Client (or aliased to Client) from @trustloopguard/sdk.
  if grep -nE "^[[:space:]]*import[[:space:]]+(\{[^}]*\bClient\b[^}]*\}|Client)[[:space:]]+from[[:space:]]+['\"]@trustloopguard/sdk['\"]" \
      "${file}" >/dev/null 2>&1; then
    if ! grep -nE "^[[:space:]]*import[[:space:]]+type[[:space:]]+" "${file}" | grep -qE "\bClient\b.*from.*@trustloopguard/sdk"; then
      while IFS= read -r line; do
        # Skip lines that are clearly type-only.
        case "${line}" in
          *"import type"*) continue ;;
        esac
        violations+=("${file}: @trustloopguard/sdk Client imported in browser-bundled code → ${line}")
      done < <(grep -nE "^[[:space:]]*import[[:space:]]+(\{[^}]*\bClient\b[^}]*\}|Client)[[:space:]]+from[[:space:]]+['\"]@trustloopguard/sdk['\"]" "${file}")
    fi
  fi

  # 3. Absolute-URL fetch — browser code must call same-origin /api/*.
  if grep -nE "fetch\([[:space:]]*[\"'\`]https?://" "${file}" >/dev/null 2>&1; then
    while IFS= read -r line; do
      violations+=("${file}: fetch() with absolute URL in browser-bundled code → ${line}")
    done < <(grep -nE "fetch\([[:space:]]*[\"'\`]https?://" "${file}")
  fi
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
  echo "apps/web browser-bundled code may only call same-origin /api/* routes." >&2
  echo "Move external calls behind a Next API route under apps/web/app/api/," >&2
  echo "or a server-only helper under apps/web/lib/server/." >&2
  echo "" >&2
  for v in "${violations[@]}"; do
    printf '  %s\n' "${v}" >&2
  done
  exit 1
fi

echo "apps/web boundary lint: OK"
