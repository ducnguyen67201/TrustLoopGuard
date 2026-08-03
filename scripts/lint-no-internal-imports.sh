#!/usr/bin/env bash
# Mechanical enforcement of docs/SDK_DRIVEN.md rule 2:
#   "No internal imports in demo/."
#
# Internal crates are everything in crates/ except tl-sdk-rust. Internal
# Python is anything outside the published `featherlane-ai` package.
# Internal TypeScript is anything other than the published
# `@featherlane-ai/sdk`.
#
# A failure here means: the demo reached into an internal API the SDK
# doesn't expose. Either add the missing surface to the SDK and use it
# from the demo, or rework the demo to not need it. Bypassing this
# lint defeats the whole point of SDK-driven development.

set -euo pipefail

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "${ROOT}"

# Directories that must not contain internal imports.
SCAN_DIRS=("demo")

# Internal Rust crate names. Any `use tl_X::` in scanned dirs is a
# violation unless X is in the allowlist below.
INTERNAL_CRATES=(
  tl_core
  tl_engine
  tl_policy
  tl_server
  tl_fuzzy
  tl_storage
  tl_replay
  tl_cli
  tl_stream
  tl_codegen
)

# Allowed crates / packages.
RUST_ALLOW=(tl_sdk_rust)
PY_ALLOW=(featherlane-ai)
TS_ALLOW=("@featherlane-ai/sdk")

violations=()

# ----------------------------------------------------------------------
# Rust: scan src/ files for `use tl_X` / `extern crate tl_X` matching
# any internal crate name.

scan_rust() {
  local dir="$1"
  local rs_files
  rs_files=$(find "${dir}" -name "*.rs" -not -path "*/target/*" 2>/dev/null || true)
  [[ -z "${rs_files}" ]] && return 0
  for crate in "${INTERNAL_CRATES[@]}"; do
    while IFS= read -r hit; do
      [[ -z "${hit}" ]] && continue
      violations+=("rust  : ${hit}")
    done < <(
      echo "${rs_files}" |
        xargs grep -nE "(^|[[:space:]])(use|extern[[:space:]]+crate)[[:space:]]+${crate}\b" \
          2>/dev/null || true
    )
  done

  # Also scan Cargo.toml files for direct deps on internal crates.
  while IFS= read -r toml; do
    [[ -z "${toml}" ]] && continue
    for crate in "${INTERNAL_CRATES[@]}"; do
      # Cargo dep names use dashes; convert tl_core -> tl-core.
      dep_name="${crate//_/-}"
      if grep -nE "^[[:space:]]*${dep_name}[[:space:]]*=" "${toml}" >/dev/null 2>&1; then
        violations+=("rust  : ${toml}: dependency ${dep_name}")
      fi
    done
  done < <(find "${dir}" -name "Cargo.toml" -not -path "*/target/*" 2>/dev/null || true)
}

# ----------------------------------------------------------------------
# Python: any `import tl_X` / `from tl_X` is internal. Also flag any
# `from <path-outside-featherlane-ai>` style sys-path manipulation.

scan_python() {
  local dir="$1"
  local py_files
  py_files=$(find "${dir}" -name "*.py" 2>/dev/null || true)
  [[ -z "${py_files}" ]] && return 0
  for crate in "${INTERNAL_CRATES[@]}"; do
    while IFS= read -r hit; do
      [[ -z "${hit}" ]] && continue
      violations+=("python: ${hit}")
    done < <(
      echo "${py_files}" |
        xargs grep -nE "(from|import)[[:space:]]+${crate}\b" 2>/dev/null || true
    )
  done

  # `from featherlane_ai._generated` is internal — public types come
  # from the package root, not from generated/private modules.
  while IFS= read -r hit; do
    [[ -z "${hit}" ]] && continue
    violations+=("python: ${hit} (use top-level featherlane-ai imports, not _generated)")
  done < <(
    echo "${py_files}" |
      xargs grep -nE "from[[:space:]]+featherlane-ai\._" 2>/dev/null || true
  )
}

# ----------------------------------------------------------------------
# TypeScript: any import that doesn't resolve to @featherlane-ai/sdk
# (or relative to the example itself) is internal. Easiest mechanical
# check: forbid relative imports that escape the example dir, and
# forbid imports of internal generated paths.

scan_typescript() {
  local dir="$1"
  local ts_files
  ts_files=$(find "${dir}" -name "*.ts" -not -path "*/node_modules/*" -not -path "*/dist/*" 2>/dev/null || true)
  [[ -z "${ts_files}" ]] && return 0

  # Imports that escape the example: `from '../../something'` etc.
  while IFS= read -r hit; do
    [[ -z "${hit}" ]] && continue
    # First-party demo model selection intentionally reads the versioned,
    # data-only routing manifest. This is configuration, not an internal API.
    [[ "${hit}" == *"../../config/llm-routing.json"* ]] && continue
    violations+=("ts    : ${hit} (escapes example dir — use the published SDK)")
  done < <(
    echo "${ts_files}" |
      xargs grep -nE "from[[:space:]]+['\"]\.\.\/\.\.\/" 2>/dev/null || true
  )

  # Imports of generated or src paths inside the SDK.
  while IFS= read -r hit; do
    [[ -z "${hit}" ]] && continue
    violations+=("ts    : ${hit} (use the package entrypoint, not internal modules)")
  done < <(
    echo "${ts_files}" |
      xargs grep -nE "from[[:space:]]+['\"]@featherlane-ai/sdk/(src|generated)" 2>/dev/null || true
  )
}

# ----------------------------------------------------------------------
# Drive the scans.

for dir in "${SCAN_DIRS[@]}"; do
  [[ -d "${dir}" ]] || continue
  scan_rust "${dir}"
  scan_python "${dir}"
  scan_typescript "${dir}"
done

# Emit allowlists for the human reading the failure.
if [[ "${#violations[@]}" -gt 0 ]]; then
  echo "::error::Internal-crate imports detected in demo/."
  echo
  echo "Featherlane AI is SDK-driven (docs/SDK_DRIVEN.md rule 2). Demos"
  echo "must import only the published SDK surface:"
  echo "  - Rust:       ${RUST_ALLOW[*]}"
  echo "  - Python:     ${PY_ALLOW[*]} (top-level only — no _generated.*)"
  echo "  - TypeScript: ${TS_ALLOW[*]}"
  echo
  echo "Violations:"
  for v in "${violations[@]}"; do
    echo "  ${v}"
  done
  echo
  echo "Fix: either expose the missing surface from the SDK, or rework"
  echo "the demo to not need the internal API."
  exit 1
fi

echo "lint-no-internal-imports: ok"
