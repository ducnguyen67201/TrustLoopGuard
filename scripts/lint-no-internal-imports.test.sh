#!/usr/bin/env bash

set -euo pipefail

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "${ROOT}"

fixture_dir="$(mktemp -d "${ROOT}/demo/shared/boundary-lint.XXXXXX")"
suffix_fixture="${fixture_dir}/suffix.ts"
comment_fixture="${fixture_dir}/comment.ts"
trap 'rm -f "${suffix_fixture}" "${comment_fixture}"; rmdir "${fixture_dir}"' EXIT

printf '%s\n' "import manifest from '../../config/llm-routing.json.bak';" >"${suffix_fixture}"
printf '%s\n' \
  "import internal from '../../crates/tl-core'; // ../../config/llm-routing.json" \
  >"${comment_fixture}"

if output="$(scripts/lint-no-internal-imports.sh 2>&1)"; then
  echo "expected boundary lint to reject lookalike imports"
  exit 1
fi

for fixture in "${suffix_fixture}" "${comment_fixture}"; do
  relative_fixture="${fixture#"${ROOT}/"}"
  if [[ "${output}" != *"${relative_fixture}"* ]]; then
    echo "boundary lint failed without reporting fixture: ${fixture}"
    exit 1
  fi
done

echo "lint-no-internal-imports tests: ok"
