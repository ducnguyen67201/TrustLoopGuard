#!/usr/bin/env bash
set -euo pipefail

if ! cargo llvm-cov --version >/dev/null 2>&1; then
  cat >&2 <<'EOF'
cargo-llvm-cov is required for backend coverage.

Install it with:
  cargo install cargo-llvm-cov

Then rerun:
  make backend-coverage
EOF
  exit 127
fi

ts_export_dir="$(mktemp -d)"
trap 'rm -rf "$ts_export_dir"' EXIT

TS_RS_EXPORT_DIR="$ts_export_dir" \
  cargo llvm-cov --locked --workspace --all-targets --no-fail-fast "$@"
