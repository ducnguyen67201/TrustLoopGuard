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
    /^permissions:[[:space:]]*(write-all|"write-all"|\047write-all\047)([[:space:]]*(#.*)?)?$/ { found = 1; next }
    /^permissions:/ { in_top_permissions = 1; next }
    in_top_permissions && /^[^ ]/ { in_top_permissions = 0 }
    in_top_permissions && /^  contents:[[:space:]]*(write|"write"|\047write\047)([[:space:]]*(#.*)?)?$/ { found = 1 }
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
      if [[ "$use" == ./* ]]; then
        continue
      fi
      if [[ "$use" == docker://* ]]; then
        if [[ "$use" =~ ^docker://[^@]+@sha256:[0-9a-fA-F]{64}$ ]]; then
          continue
        fi
        report "$workflow job '$job' uses mutable Docker action ref '$use'"
        continue
      fi
      ref="${use##*@}"
      if [[ ${#ref} -ne 40 || ! "$ref" =~ ^[0-9a-f]+$ ]]; then
        report "$workflow job '$job' uses mutable action ref '$use'"
      fi
    done < <(
      printf '%s\n' "$job_block" |
        sed -nE 's/^[[:space:]]*-?[[:space:]]*uses:[[:space:]]*["'\'']?([^[:space:]#"'\'']+)["'\'']?.*$/\1/p'
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
      in_jobs && /^    permissions:[[:space:]]*(write-all|"write-all"|\047write-all\047)([[:space:]]*(#.*)?)?$/ { print current }
      in_jobs && /^      contents:[[:space:]]*(write|"write"|\047write\047)([[:space:]]*(#.*)?)?$/ { print current }
    ' "$workflow"
  )
done

graphify_workflow=".github/workflows/graphify.yml"
graphify_requirements=".github/graphify-requirements.txt"
graphify_hashes=".github/graphify-wheels.sha256"

if [[ -f "$graphify_workflow" ]]; then
  if [[ ! -f "$graphify_requirements" || ! -f "$graphify_hashes" ]]; then
    report "$graphify_workflow must keep its exact requirements and wheel hash manifest"
  else
    if awk '
      /^[[:space:]]*(#|$)/ { next }
      !/^[A-Za-z0-9_.-]+==[A-Za-z0-9_.+!-]+$/ { invalid = 1 }
      END { exit invalid ? 0 : 1 }
    ' "$graphify_requirements"; then
      report "$graphify_requirements contains a dependency without an exact version"
    fi

    requirement_count="$(
      awk '/^[A-Za-z0-9_.-]+==[A-Za-z0-9_.+!-]+$/ { count += 1 } END { print count + 0 }' \
        "$graphify_requirements"
    )"
    manifest_count=0
    manifest_unique_count=0
    manifest_wheels_seen=$'\n'
    while read -r hash wheel extra; do
      [[ -n "$hash" ]] || continue
      manifest_count=$((manifest_count + 1))
      if [[ ! "$hash" =~ ^[0-9a-f]{64}$ ||
        ! "$wheel" =~ ^[A-Za-z0-9][A-Za-z0-9_.+-]*\.whl$ ||
        -n "${extra:-}" ]]; then
        report "$graphify_hashes contains an invalid wheel hash entry"
        continue
      fi
      if [[ "$manifest_wheels_seen" == *$'\n'"$wheel"$'\n'* ]]; then
        report "$graphify_hashes contains duplicate wheel '$wheel'"
      else
        manifest_wheels_seen+="$wheel"$'\n'
        manifest_unique_count=$((manifest_unique_count + 1))
      fi
    done < "$graphify_hashes"
    if [[ "$manifest_unique_count" -ne "$manifest_count" ]]; then
      report "$graphify_hashes must contain safe, unique wheel basenames"
    fi
    if [[ "$manifest_count" -ne "$requirement_count" ]]; then
      report "$graphify_hashes must contain one wheel for every exact Graphify requirement"
    fi

    graphify_install_step="$(
      awk '
        /^      - name:[[:space:]]*install graphify[[:space:]]*$/ {
          in_install = 1
        }
        in_install && /^      - (name|uses):/ && !/install graphify/ { exit }
        in_install { print }
      ' "$graphify_workflow"
    )"
    if [[ -z "$graphify_install_step" ]]; then
      report "$graphify_workflow is missing the Graphify install step"
    else
      graphify_install_commands="$(
        sed -E '/^[[:space:]]*#/d; s/[[:space:]]+#.*$//' <<<"$graphify_install_step"
      )"
      graphify_download_command="$(
        awk '
          /python[[:space:]]+-m[[:space:]]+pip[[:space:]]+download/ {
            in_command = 1
          }
          in_command { print }
          in_command && !/\\[[:space:]]*$/ { exit }
        ' <<<"$graphify_install_commands"
      )"
      graphify_pip_install_command="$(
        awk '
          /python"?[[:space:]]+-m[[:space:]]+pip[[:space:]]+install/ {
            in_command = 1
          }
          in_command { print }
          in_command && !/\\[[:space:]]*$/ { exit }
        ' <<<"$graphify_install_commands"
      )"
      if ! grep -Eq \
        'sha256sum[[:space:]]+--check[[:space:]]+("\$manifest"|.*graphify-wheels\.sha256)' \
        <<<"$graphify_install_commands"; then
        report "$graphify_workflow Graphify install step does not execute manifest verification"
      fi
      if ! grep -Eq \
        'cmp[[:space:]]+-s[[:space:]]+"\$manifest_names"[[:space:]]+"\$downloaded_names"' \
        <<<"$graphify_install_commands"; then
        report "$graphify_workflow Graphify install step does not compare the exact wheel set"
      fi
      for required_token in \
        "--no-deps" \
        ".github/graphify-requirements.txt"; do
        if ! grep -Fq -- "$required_token" <<<"$graphify_download_command"; then
          report "$graphify_workflow Graphify download command is missing '$required_token'"
        fi
      done
      for required_token in \
        "--no-index" \
        "--no-deps" \
        ".github/graphify-requirements.txt"; do
        if ! grep -Fq -- "$required_token" <<<"$graphify_pip_install_command"; then
          report "$graphify_workflow Graphify pip install command is missing '$required_token'"
        fi
      done
      if ! grep -Fq -- ".github/graphify-wheels.sha256" <<<"$graphify_install_commands"; then
        report "$graphify_workflow Graphify install step does not reference the wheel manifest"
      fi
    fi
  fi
fi

exit "$status"
