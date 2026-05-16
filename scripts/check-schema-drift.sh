#!/usr/bin/env bash
# Detect drift between the Rust-owned Diesel schema and the live database.
#
# Source of truth: crates/tl-storage/src/schema.rs. The live DB is the
# product of running every migration in crates/tl-storage/migrations.
# This script asks Diesel to regenerate the schema from the live DB and
# diffs it against the committed file. Any difference is real drift —
# either a migration changed the DB without an accompanying schema.rs
# update, or schema.rs was hand-edited without a migration.
#
# Usage:
#   scripts/check-schema-drift.sh                 # default DB at localhost:5432
#   DATABASE_URL=postgres://... scripts/check-schema-drift.sh

set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

SCHEMA_FILE="crates/tl-storage/src/schema.rs"
DATABASE_URL="${DATABASE_URL:-postgres://tl:tl@localhost:5432/tl}"

if ! command -v diesel >/dev/null 2>&1; then
  cat >&2 <<EOF
diesel CLI not found. Install it with:

  cargo install --locked diesel_cli --no-default-features --features postgres

(The Postgres client library 'libpq' must be available — on macOS:
 brew install libpq && brew link --force libpq)
EOF
  exit 2
fi

if ! diesel print-schema --database-url "$DATABASE_URL" >/dev/null 2>&1; then
  cat >&2 <<EOF
Cannot connect to the database at:

  $DATABASE_URL

Start the local stack first:

  docker compose up -d db server
  # (server applies migrations on boot)

Or point DATABASE_URL at a different instance.
EOF
  exit 2
fi

TMP_LIVE="$(mktemp)"
TMP_FILE="$(mktemp)"
trap 'rm -f "$TMP_LIVE" "$TMP_FILE"' EXIT

# Normalize both sides:
# - drop the "// @generated ..." header diesel emits (schema.rs doesn't have it)
# - drop trailing blank lines so EOF newlines don't show up as drift
normalize() {
  sed -E '/^\/\/ @generated/d' "$1" \
    | awk 'NF {p=1} p' \
    | sed -e :a -e '/^$/{$d;N;ba' -e '}'
}

diesel print-schema --database-url "$DATABASE_URL" > "$TMP_LIVE"

normalize "$SCHEMA_FILE" > "$TMP_FILE.committed"
normalize "$TMP_LIVE"    > "$TMP_FILE.live"

if diff -u \
    --label "committed: $SCHEMA_FILE" "$TMP_FILE.committed" \
    --label "live:      $DATABASE_URL" "$TMP_FILE.live"; then
  echo "OK: $SCHEMA_FILE matches the live database."
  exit 0
fi

cat >&2 <<EOF

DRIFT detected between $SCHEMA_FILE and the live database.

Resolve by picking the side that should win:

  1. Rust is the source of truth (most common):
     Write a migration in crates/tl-storage/migrations/ that brings the DB
     in line with schema.rs, then re-run this check.

  2. The DB is correct and schema.rs is stale:
     diesel print-schema --database-url "$DATABASE_URL" > $SCHEMA_FILE
     cargo fmt -p tl-storage
EOF
exit 1
