# syntax=docker/dockerfile:1.7
#
# tl-server multi-stage Dockerfile.
#
# Layout:
#   1. chef    — base image with cargo-chef installed
#   2. planner — produces recipe.json (dep graph hash) from the workspace
#   3. builder — cooks deps from the recipe (cached), then builds tl-server
#   4. runtime — distroless cc image, just the binary + baked policies
#
# Why cargo-chef: cold builds take ~10 min for the dep graph; cargo-chef
# lets us cache that as a separate layer keyed on Cargo.{toml,lock}, so
# touching a .rs file only re-runs the final ~30s compile.
#
# Why distroless: ~30 MB runtime, no shell, non-root by default. The cc
# variant ships glibc + libgcc, which is what reqwest's rustls-tls and
# Diesel's Postgres client need. No openssl is pulled in.

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
# Build-time deps for Diesel + reqwest. No openssl is needed, but
# pkg-config and a C linker are still required by some sys crates.
RUN apt-get update && apt-get install -y --no-install-recommends \
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
FROM gcr.io/distroless/cc-debian12:nonroot AS runtime

# Bake the v0 policy bundle into a known path. Compose mounts a host
# volume on top of this for the dev edit loop; in pure `docker run` mode
# the baked copy is what serves.
COPY --chown=nonroot:nonroot policies /etc/trustloopguard/policies

COPY --from=builder /app/target/release/tl-server /usr/local/bin/tl-server

ENV TL_POLICY_DIR=/etc/trustloopguard/policies \
    RUST_LOG=info

EXPOSE 8080

# Distroless has no shell, so HEALTHCHECK via wget/curl isn't an option.
# Compose declares the healthcheck against /health using `wget` from the
# postgres-alpine image's shell — see docker-compose.yml. We still set a
# minimal HEALTHCHECK NONE marker so callers know the image opts out and
# defers to the orchestrator.
HEALTHCHECK NONE

ENTRYPOINT ["/usr/local/bin/tl-server"]
