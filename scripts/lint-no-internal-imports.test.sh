#!/usr/bin/env bash

set -euo pipefail

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "${ROOT}"

fixture=""
fixture_dir=""
cleanup() {
  if [[ -n "${fixture}" ]]; then
    rm -f "${fixture}"
  fi
  if [[ -n "${fixture_dir}" ]]; then
    rmdir "${fixture_dir}" 2>/dev/null || true
  fi
}
trap cleanup EXIT

assert_rejected() {
  local source="$1"
  local relative_fixture
  fixture_dir="$(mktemp -d "${ROOT}/demo/shared/boundary-lint.XXXXXX")"
  fixture="${fixture_dir}/fixture.ts"
  relative_fixture="${fixture#"${ROOT}/"}"
  printf '%s\n' "${source}" >"${fixture}"

  if output="$(scripts/lint-no-internal-imports.sh 2>&1)"; then
    echo "expected boundary lint to reject: ${source}"
    return 1
  fi
  if [[ "${output}" != *"${relative_fixture}"* ]]; then
    echo "boundary lint failed without reporting fixture: ${fixture}"
    return 1
  fi

  rm -f "${fixture}"
  rmdir "${fixture_dir}"
  fixture=""
  fixture_dir=""
}

assert_rejected "import manifest from '../../config/llm-routing.json.bak';"
assert_rejected "import internal from '../../crates/tl-core'; // ../../config/llm-routing.json"

echo "lint-no-internal-imports tests: ok"
