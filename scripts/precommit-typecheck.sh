#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "${ROOT}"

all_packages=0
packages=()

add_package() {
  local pkg="$1"
  local existing
  for existing in "${packages[@]+"${packages[@]}"}"; do
    [[ "${existing}" == "${pkg}" ]] && return 0
  done
  packages+=("${pkg}")
}

while IFS= read -r path; do
  case "${path}" in
    package.json|pnpm-lock.yaml|pnpm-workspace.yaml|tsconfig.base.json)
      all_packages=1
      ;;
    apps/web/*)
      add_package "web"
      ;;
    apps/docs/*)
      add_package "docs"
      ;;
    apps/marketing/*)
      add_package "marketing"
      ;;
    demo/*)
      add_package "@trustloopguard/demo"
      ;;
    sdks/typescript/*)
      add_package "@trustloopguard/sdk"
      ;;
  esac
done < <(git diff --cached --name-only --diff-filter=ACMR)

if (( all_packages == 1 )); then
  pnpm typecheck
  exit 0
fi

if (( ${#packages[@]} == 0 )); then
  echo "typecheck: no staged TypeScript package changes"
  exit 0
fi

for pkg in "${packages[@]}"; do
  pnpm --filter "${pkg}" typecheck
done
