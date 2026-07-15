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

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

# Diesel emits tables alphabetically, columns in database-creation order,
# Postgres enum names as custom SQL types, and legacy compatibility
# artifacts that the Rust data layer deliberately does not expose. None of
# those presentation details is schema drift. Canonicalize the table contract
# to primary keys plus sorted column names/types before comparing it.
canonicalize() {
  awk '
    function normalize_type(type) {
      if (type == "ApiKeyStatus" || type == "InviteStatus" ||
          type == "OrganizationRole" || type == "WorkspaceRole") {
        return "Text"
      }
      if (type == "Bytea") return "Binary"
      return type
    }
    /^diesel::table![[:space:]]*\{/ { in_table = 1; table = ""; next }
    in_table && table == "" {
      header = $0
      sub(/^[[:space:]]*/, "", header)
      if (header ~ /^[A-Za-z_][A-Za-z0-9_]*[[:space:]]*\(/) {
        table = header
        sub(/[[:space:]]*\(.*/, "", table)
        primary_key = header
        sub(/^[^(]*\(/, "", primary_key)
        sub(/\).*/, "", primary_key)
        gsub(/[[:space:]]/, "", primary_key)
        print "T|" table "|" primary_key
      }
      next
    }
    in_table && table != "" && /->[[:space:]]*/ {
      line = $0
      sub(/^[[:space:]]*/, "", line)
      column = line
      sub(/[[:space:]]*->.*/, "", column)
      type = line
      sub(/^.*->[[:space:]]*/, "", type)
      sub(/,[[:space:]]*$/, "", type)
      gsub(/[[:space:]]/, "", type)
      print "C|" table "|" column "|" normalize_type(type)
      next
    }
    in_table && table != "" && /^[[:space:]]*\}[[:space:]]*$/ {
      in_table = 0
      table = ""
    }
  ' "$1" | LC_ALL=C sort
}

diesel print-schema --database-url "$DATABASE_URL" > "$TMP_DIR/live.rs"
canonicalize "$SCHEMA_FILE" > "$TMP_DIR/committed.contract"
canonicalize "$TMP_DIR/live.rs" > "$TMP_DIR/live.all.contract"

# Filter the live contract to Rust-owned tables. Fresh databases also contain
# quoted pre-Rust compatibility tables, migration bookkeeping, and the trace
# partition child; migrations intentionally preserve those while schema.rs
# intentionally hides them.
awk -F'|' '
  NR == FNR { if ($1 == "T") keep[$2] = 1; next }
  keep[$2]
' "$TMP_DIR/committed.contract" "$TMP_DIR/live.all.contract" \
  > "$TMP_DIR/live.contract"

awk -F'|' '
  NR == FNR { if ($1 == "T") committed[$2] = 1; next }
  $1 == "T" && !committed[$2] &&
    $2 != "Agent" && $2 != "Escalations" && $2 != "Traces" &&
    $2 != "Traces_default" && $2 != ("_" "s" "q" "l" "x_migrations") &&
    $2 != "traces_default" { print $2 }
' "$TMP_DIR/committed.contract" "$TMP_DIR/live.all.contract" \
  > "$TMP_DIR/untracked-tables"

if [[ -s "$TMP_DIR/untracked-tables" ]]; then
  echo "Live database contains tables missing from $SCHEMA_FILE:" >&2
  sed 's/^/  - /' "$TMP_DIR/untracked-tables" >&2
  exit 1
fi

if diff -u \
    --label "committed: $SCHEMA_FILE" "$TMP_DIR/committed.contract" \
    --label "live:      $DATABASE_URL" "$TMP_DIR/live.contract"; then
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
