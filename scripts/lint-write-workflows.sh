#!/usr/bin/env bash
set -euo pipefail

status=0

report() {
  printf 'workflow security lint: %s\n' "$*" >&2
  status=1
}

for workflow in .github/workflows/*.yml .github/workflows/*.yaml; do
  [[ -f "$workflow" ]] || continue

  if awk '
    /^permissions:/ { in_top_permissions = 1; next }
    in_top_permissions && /^[^ ]/ { in_top_permissions = 0 }
    in_top_permissions && /^  contents:[[:space:]]*write([[:space:]]|$)/ { found = 1 }
    END { exit found ? 0 : 1 }
  ' "$workflow"; then
    report "$workflow grants top-level contents:write; scope write access to one job"
  fi

  while IFS= read -r job; do
    [[ -n "$job" ]] || continue
    job_block="$(
      awk -v target="$job" '
        /^jobs:/ { in_jobs = 1; next }
        in_jobs && /^[^ ]/ { in_jobs = 0 }
        in_jobs && /^  [A-Za-z0-9_-]+:[[:space:]]*(#.*)?$/ {
          current = $1
          sub(/:$/, "", current)
        }
        in_jobs && current == target { print }
      ' "$workflow"
    )"

    while IFS= read -r use; do
      [[ -n "$use" ]] || continue
      if [[ "$use" == ./* || "$use" == docker://* ]]; then
        continue
      fi
      ref="${use##*@}"
      if [[ ${#ref} -ne 40 || ! "$ref" =~ ^[0-9a-f]+$ ]]; then
        report "$workflow job '$job' uses mutable action ref '$use'"
      fi
    done < <(
      printf '%s\n' "$job_block" |
        sed -nE 's/^[[:space:]]*-?[[:space:]]*uses:[[:space:]]*["'\'']?([^"'\'']+@[^[:space:]#"'\'']+)["'\'']?.*$/\1/p'
    )

    if printf '%s\n' "$job_block" |
      grep -Eq '(^|[[:space:]])(pipx?|npm|pnpm|yarn|bun|cargo)[[:space:]]+install([[:space:]]|$)'; then
      report "$workflow job '$job' installs executable dependencies while holding contents:write"
    fi
  done < <(
    awk '
      /^jobs:/ { in_jobs = 1; next }
      in_jobs && /^[^ ]/ { in_jobs = 0 }
      in_jobs && /^  [A-Za-z0-9_-]+:[[:space:]]*(#.*)?$/ {
        current = $1
        sub(/:$/, "", current)
      }
      in_jobs && /^      contents:[[:space:]]*write([[:space:]]|$)/ { print current }
    ' "$workflow"
  )
done

exit "$status"
