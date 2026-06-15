#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "${ROOT}"

REMOTE="${1:-origin}"

changed_files=()
secret_files=()
packages=()
all_ts_packages=0
rust_changed=0
web_changed=0
sdk_ts_changed=0
boundary_changed=0
contract_changed=0

run() {
  echo "+ $*"
  "$@"
}

add_package() {
  local pkg="$1"
  local existing
  for existing in "${packages[@]+"${packages[@]}"}"; do
    [[ "${existing}" == "${pkg}" ]] && return 0
  done
  packages+=("${pkg}")
}

ref_exists() {
  git rev-parse --verify --quiet "$1" >/dev/null
}

detect_base_ref() {
  local upstream branch remote_head candidate

  if upstream="$(git rev-parse --abbrev-ref --symbolic-full-name '@{u}' 2>/dev/null)"; then
    echo "${upstream}"
    return 0
  fi

  branch="$(git branch --show-current 2>/dev/null || true)"
  if [[ -n "${branch}" ]] && ref_exists "refs/remotes/${REMOTE}/${branch}"; then
    echo "refs/remotes/${REMOTE}/${branch}"
    return 0
  fi

  if remote_head="$(git symbolic-ref --quiet --short "refs/remotes/${REMOTE}/HEAD" 2>/dev/null)"; then
    echo "${remote_head}"
    return 0
  fi

  for candidate in "refs/remotes/${REMOTE}/main" "refs/remotes/${REMOTE}/master" "main" "master"; do
    if ref_exists "${candidate}"; then
      echo "${candidate}"
      return 0
    fi
  done

  return 1
}

collect_changed_files() {
  local base_ref merge_base

  if base_ref="$(detect_base_ref)"; then
    merge_base="$(git merge-base "${base_ref}" HEAD 2>/dev/null || true)"
    if [[ -n "${merge_base}" ]]; then
      git diff --name-only --diff-filter=ACMR "${merge_base}..HEAD"
    else
      git diff --name-only --diff-filter=ACMR "${base_ref}..HEAD"
    fi
    return 0
  fi

  git diff-tree --no-commit-id --name-only -r --diff-filter=ACMR HEAD
}

while IFS= read -r path; do
  [[ -n "${path}" ]] || continue
  changed_files+=("${path}")
done < <(collect_changed_files)

if (( ${#changed_files[@]} == 0 )); then
  echo "prepush: no branch changes detected"
  exit 0
fi

for path in "${changed_files[@]}"; do
  if [[ -f "${path}" ]]; then
    case "${path}" in
      node_modules/*|target/*|*/.next/*|*/dist/*|*/coverage/*|graphify-out/*)
        ;;
      *)
        secret_files+=("${path}")
        ;;
    esac
  fi

  case "${path}" in
    package.json|pnpm-lock.yaml|pnpm-workspace.yaml|tsconfig.base.json)
      all_ts_packages=1
      ;;
    apps/web/*)
      add_package "web"
      web_changed=1
      boundary_changed=1
      ;;
    apps/docs/*)
      add_package "docs"
      ;;
    apps/marketing/*)
      add_package "marketing"
      ;;
    demo/*)
      add_package "@trustloopguard/demo"
      boundary_changed=1
      ;;
    sdks/typescript/*)
      add_package "@trustloopguard/sdk"
      sdk_ts_changed=1
      boundary_changed=1
      ;;
  esac

  case "${path}" in
    Cargo.toml|Cargo.lock|rust-toolchain.toml|crates/*)
      rust_changed=1
      boundary_changed=1
      ;;
  esac

  case "${path}" in
    crates/tl-core/*|crates/tl-codegen/*|docs/openapi.yaml|policies/*.schema.json|sdks/typescript/src/generated/*|sdks/python/src/trustloopguard/_generated/*)
      contract_changed=1
      ;;
  esac
done

if (( ${#secret_files[@]} > 0 )); then
  run pnpm exec secretlint --no-glob "${secret_files[@]}"
else
  echo "secretlint: no changed files to scan"
fi

if (( all_ts_packages == 1 )); then
  run pnpm typecheck
elif (( ${#packages[@]} > 0 )); then
  for pkg in "${packages[@]}"; do
    run pnpm --filter "${pkg}" typecheck
  done
else
  echo "typecheck: no changed TypeScript package files"
fi

if (( web_changed == 1 )); then
  run pnpm --filter web test
fi

if (( sdk_ts_changed == 1 )); then
  run pnpm --filter @trustloopguard/sdk test
fi

if (( rust_changed == 1 )); then
  run cargo fmt --all -- --check
  run cargo check --locked --workspace --all-targets
fi

if (( boundary_changed == 1 )); then
  run pnpm lint:boundaries
fi

if (( contract_changed == 1 )); then
  run pnpm codegen:check
fi
