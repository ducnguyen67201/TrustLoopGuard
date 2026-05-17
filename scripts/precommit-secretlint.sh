#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "${ROOT}"

files=()
while IFS= read -r path; do
  [[ -f "${path}" ]] || continue
  case "${path}" in
    node_modules/*|target/*|*/.next/*|*/dist/*|*/coverage/*|graphify-out/*)
      continue
      ;;
  esac
  files+=("${path}")
done < <(git diff --cached --name-only --diff-filter=ACMR)

if (( ${#files[@]} == 0 )); then
  echo "secretlint: no staged files"
  exit 0
fi

pnpm exec secretlint --no-glob "${files[@]}"
