#!/usr/bin/env sh
#
# TrustLoopGuard one-line installer.
#
# Usage:
#   curl -sSL https://raw.githubusercontent.com/ducnguyen67201/TrustLoopGuard/main/docker/install.sh | sh
#
# What it does:
#   - Verifies Docker is installed and running.
#   - Drops docker-compose.yml + .env.example into ./trustloopguard/
#     (override directory with TLG_DIR=...).
#   - Prints next steps. Does NOT run `docker compose up` for you —
#     the operator should see the command before it runs.
#
# Idempotent: re-running overwrites compose.yml, leaves any existing
# .env untouched, and leaves user-edited policies/ alone.
#
# Audit this script before piping to sh:
#   curl -sSL https://raw.githubusercontent.com/.../docker/install.sh

set -eu

REPO="${TLG_REPO:-ducnguyen67201/TrustLoopGuard}"
BRANCH="${TLG_BRANCH:-main}"
DIR="${TLG_DIR:-trustloopguard}"
RAW="https://raw.githubusercontent.com/${REPO}/${BRANCH}"

bold() { printf '\033[1m%s\033[0m\n' "$*"; }
info() { printf '  %s\n' "$*"; }
err()  { printf '\033[31merror:\033[0m %s\n' "$*" >&2; exit 1; }

# --- Pre-flight ---------------------------------------------------------------

command -v docker >/dev/null 2>&1 || err "Docker is not installed. See https://docs.docker.com/get-docker/"
docker info >/dev/null 2>&1 || err "Docker daemon is not running. Start Docker Desktop or your docker service."

# `docker compose` (v2 plugin) is required; the legacy `docker-compose` v1 is not supported.
docker compose version >/dev/null 2>&1 || err "Docker Compose v2 is required. Update Docker Desktop or install the compose plugin."

# `curl` is what we use; some minimal containers ship only wget. Detect both.
if command -v curl >/dev/null 2>&1; then
    fetch() { curl -fsSL "$1" -o "$2"; }
elif command -v wget >/dev/null 2>&1; then
    fetch() { wget -qO "$2" "$1"; }
else
    err "Need curl or wget to download files."
fi

# --- Layout -------------------------------------------------------------------

bold "Installing TrustLoopGuard into ./${DIR}"

mkdir -p "${DIR}"
cd "${DIR}"

# compose.yml is always overwritten — it's how upgrades happen.
info "fetching docker-compose.yml"
fetch "${RAW}/docker-compose.yml" docker-compose.yml

# .env.example is always overwritten so docs stay in sync.
info "fetching .env.example"
fetch "${RAW}/.env.example" .env.example

# .env is left alone if the user already created one.
if [ ! -f .env ]; then
    info "creating empty .env (override defaults from .env.example as needed)"
    : > .env
else
    info ".env already exists — leaving it alone"
fi

# --- Done ---------------------------------------------------------------------

cat <<EOF

$(bold 'TrustLoopGuard ready in ./'"${DIR}")

  cd ${DIR}
  docker compose up

Then open http://localhost:3000

Optional configuration lives in .env (see .env.example).
Stop with: docker compose down
Wipe data: docker compose down -v
EOF
