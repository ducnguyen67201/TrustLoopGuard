# syntax=docker/dockerfile:1.7
#
# tl-server multi-stage Dockerfile.
#
# Layout:
#   1. chef    — base image with cargo-chef installed
#   2. planner — produces recipe.json (dep graph hash) from the workspace
#   3. builder — cooks deps from the recipe (cached), then builds tl-server
#   4. runtime — slim Debian image, just runtime libs + binary + baked policies
#
# Why cargo-chef: cold builds take ~10 min for the dep graph; cargo-chef
# lets us cache that as a separate layer keyed on Cargo.{toml,lock}, so
# touching a .rs file only re-runs the final ~30s compile.
#
# Why Debian slim runtime: Diesel's Postgres client links dynamically
# against libpq. The runtime image must include libpq5 and its transitive
# system libraries, while still running as a non-root user.

ARG RUST_VERSION=1.88
ARG DEBIAN_VERSION=bookworm

# -----------------------------------------------------------------------------
FROM rust:${RUST_VERSION}-slim-${DEBIAN_VERSION} AS chef
WORKDIR /app
RUN cargo install cargo-chef --locked --version ^0.1

# -----------------------------------------------------------------------------
FROM chef AS planner
# Copy only the manifest data needed to compute the dep graph. cargo-chef
# walks the workspace, so we need every Cargo.toml, but no source files.
COPY Cargo.toml Cargo.lock rust-toolchain.toml clippy.toml rustfmt.toml ./
COPY crates ./crates
COPY apps/example-rust ./apps/example-rust
RUN cargo chef prepare --recipe-path recipe.json

# -----------------------------------------------------------------------------
FROM chef AS builder
# Build-time deps for Diesel + reqwest. libpq-dev provides pg_config and
# the libpq linker target required by Diesel's Postgres feature.
RUN apt-get update && apt-get install -y --no-install-recommends \
    libpq-dev \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

# Cook the cached dep layer first.
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json -p tl-server --features postgres

# Now bring in the actual sources and build only tl-server.
COPY Cargo.toml Cargo.lock rust-toolchain.toml clippy.toml rustfmt.toml ./
COPY crates ./crates
COPY apps/example-rust ./apps/example-rust
RUN cargo build --release -p tl-server --features postgres \
    && strip target/release/tl-server

# -----------------------------------------------------------------------------
FROM debian:${DEBIAN_VERSION}-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libpq5 \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --system --gid 10001 tlg \
    && useradd --system --uid 10001 --gid tlg --home-dir /nonexistent --shell /usr/sbin/nologin tlg

# Bake the v0 policy bundle into a known path. Compose mounts a host
# volume on top of this for the dev edit loop; in pure `docker run` mode
# the baked copy is what serves.
COPY --chown=tlg:tlg policies /etc/trustloopguard/policies

COPY --from=builder /app/target/release/tl-server /usr/local/bin/tl-server

USER tlg:tlg

ENV TL_POLICY_DIR=/etc/trustloopguard/policies \
    RUST_LOG=info

EXPOSE 8080

HEALTHCHECK NONE

ENTRYPOINT ["/usr/local/bin/tl-server"]
