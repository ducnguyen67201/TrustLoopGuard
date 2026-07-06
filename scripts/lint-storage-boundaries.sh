#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

SELF="scripts/lint-storage-boundaries.sh"
SQL_FILE_ALLOWLIST='^crates/tl-storage/migrations/[0-9_]+_[^/]+/(up|down)\.sql$'
# Local demo fixtures may use SQLite directly to mimic a customer backend.
# Durable TrustLoopGuard product/runtime storage must still go through tl-storage.
RAW_QUERY_FILE_ALLOWLIST='^demo/stripe-refund-agent/order-db\.ts$'

fail() {
  printf '%s\n' "$1" >&2
  exit 1
}

tracked_files() {
  git ls-files
}

legacy_client_lower="s""qlx"
legacy_client_upper="S""QLx"

legacy_db_hits="$(
  tracked_files \
    | grep -v -F "$SELF" \
    | xargs grep -InE "${legacy_client_lower}|${legacy_client_upper}" 2>/dev/null || true
)"

if [[ -n "$legacy_db_hits" ]]; then
  printf '%s\n' "Legacy Rust database-client references are not allowed in the Diesel-backed workspace:" >&2
  printf '%s\n' "$legacy_db_hits" >&2
  exit 1
fi

unexpected_sql_files="$(
  tracked_files \
    | grep -E '\.sql$' \
    | grep -Ev "$SQL_FILE_ALLOWLIST" || true
)"

if [[ -n "$unexpected_sql_files" ]]; then
  printf '%s\n' "SQL files are only allowed for Diesel migrations under crates/tl-storage/migrations:" >&2
  printf '%s\n' "$unexpected_sql_files" >&2
  exit 1
fi

raw_query_hits="$(
  tracked_files \
    | grep -E '\.(rs|ts|tsx|js|jsx)$' \
    | grep -v -F "$SELF" \
    | grep -Ev "$RAW_QUERY_FILE_ALLOWLIST" \
    | xargs grep -InE 'diesel::sql_query|sql_query|sql`|"(SELECT|INSERT INTO|UPDATE|DELETE FROM|CREATE TABLE|ALTER TABLE|DROP TABLE)\b|'\''(SELECT|INSERT INTO|UPDATE|DELETE FROM|CREATE TABLE|ALTER TABLE|DROP TABLE)\b' 2>/dev/null || true
)"

if [[ -n "$raw_query_hits" ]]; then
  printf '%s\n' "Raw SQL queries are not allowed in application code. Use Diesel's typed query DSL instead:" >&2
  printf '%s\n' "$raw_query_hits" >&2
  exit 1
fi

fail_if_lock_missing_diesel="$(
  if [[ -f Cargo.lock ]] && ! grep -q 'name = "diesel"' Cargo.lock; then
    printf '%s\n' "Cargo.lock does not contain Diesel packages; regenerate it with cargo."
  fi
)"

if [[ -n "$fail_if_lock_missing_diesel" ]]; then
  fail "$fail_if_lock_missing_diesel"
fi
