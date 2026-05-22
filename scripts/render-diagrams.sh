#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SOURCE_DIR="$ROOT_DIR/docs/diagrams"
CONCEPT_OUTPUT_DIR="$ROOT_DIR/docs/concept/assets"
DOCS_APP_OUTPUT_DIR="$ROOT_DIR/apps/docs/public/diagrams"

if ! command -v d2 >/dev/null 2>&1; then
  cat >&2 <<'EOF'
Missing required command: d2

Install D2, then rerun this command:
  brew install d2

Alternative official install methods:
  https://d2lang.com/tour/install/
EOF
  exit 127
fi

mkdir -p "$CONCEPT_OUTPUT_DIR" "$DOCS_APP_OUTPUT_DIR"

for source in "$SOURCE_DIR"/*.d2; do
  name="$(basename "$source" .d2)"
  d2 --theme 4 --layout elk --pad 40 --scale 1 "$source" "$CONCEPT_OUTPUT_DIR/$name.svg"
  cp "$CONCEPT_OUTPUT_DIR/$name.svg" "$DOCS_APP_OUTPUT_DIR/$name.svg"
done
