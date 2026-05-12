#!/usr/bin/env bash
# Mechanical enforcement for public HTTP contract ownership:
# request/response/schema DTOs must live in tl-core, not tl-server route
# modules. tl-server owns transport, parsing, auth, and state wiring only.

set -euo pipefail

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "${ROOT}"

violations=()

while IFS= read -r hit; do
  [[ -z "${hit}" ]] && continue
  violations+=("${hit}")
done < <(
  grep -R -nE "utoipa::ToSchema|derive\\([^)]*ToSchema" crates/tl-server/src \
    --include "*.rs" 2>/dev/null || true
)

if [[ "${#violations[@]}" -gt 0 ]]; then
  echo "::error::tl-server defines OpenAPI schema DTOs directly."
  echo
  echo "Public API contract types must live in crates/tl-core so Rust,"
  echo "OpenAPI, Python, and TypeScript generation share one source of truth."
  echo "Move the DTO to tl-core and import it from the route module instead."
  echo
  echo "Violations:"
  for v in "${violations[@]}"; do
    echo "  - ${v}"
  done
  exit 1
fi

echo "ok: public API schema DTOs live outside tl-server"
