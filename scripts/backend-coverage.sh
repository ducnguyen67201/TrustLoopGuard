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

cargo llvm-cov --locked --workspace --all-targets --no-fail-fast "$@"
