#!/usr/bin/env bash

set -euo pipefail

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "${ROOT}"

fixture=""
cleanup() {
  if [[ -n "${fixture}" ]]; then
    rm -f "${fixture}"
  fi
}
trap cleanup EXIT

assert_rejected() {
  local source="$1"
  fixture="$(mktemp "${ROOT}/demo/shared/boundary-lint.XXXXXX.ts")"
  printf '%s\n' "${source}" >"${fixture}"

  if output="$(scripts/lint-no-internal-imports.sh 2>&1)"; then
    echo "expected boundary lint to reject: ${source}"
    return 1
  fi
  if [[ "${output}" != *"${fixture}"* ]]; then
    echo "boundary lint failed without reporting fixture: ${fixture}"
    return 1
  fi

  rm -f "${fixture}"
  fixture=""
}

assert_rejected "import manifest from '../../config/llm-routing.json.bak';"
assert_rejected "import internal from '../../crates/tl-core'; // ../../config/llm-routing.json"

echo "lint-no-internal-imports tests: ok"
