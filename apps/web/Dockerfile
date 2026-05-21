# syntax=docker/dockerfile:1.7
#
# tl-web (Next.js dashboard) multi-stage Dockerfile.
#
# Layout:
#   1. deps     — install pnpm workspace deps, cached on lockfile changes
#   2. builder  — copy sources, run `pnpm --filter web exec next build` (standalone)
#   3. runtime  — node:22-alpine, copy .next/standalone bundle, run server.js
#
# The standalone output is the whole point: Next traces only the modules
# it actually uses and emits a self-contained server. The runtime stage
# never runs `pnpm install` and contains no dev dependencies.
#
# @trustloopguard/sdk lives in the workspace and is consumed source-first
# via `transpilePackages` in next.config.ts — Next compiles it through
# SWC during `pnpm --filter web build`, so there's no separate SDK build
# step here.

ARG NODE_VERSION=22
ARG ALPINE_VERSION=3.20

# -----------------------------------------------------------------------------
FROM node:${NODE_VERSION}-alpine${ALPINE_VERSION} AS deps
WORKDIR /app
RUN corepack enable

# Copy only the manifest data needed by `pnpm install` so this layer
# caches on lockfile changes, not source changes.
COPY package.json pnpm-workspace.yaml pnpm-lock.yaml tsconfig.base.json ./
COPY apps/web/package.json ./apps/web/
COPY sdks/typescript/package.json ./sdks/typescript/

RUN PNPM_HOME=/pnpm pnpm config set store-dir /pnpm/store \
 && pnpm install --frozen-lockfile --ignore-scripts

# -----------------------------------------------------------------------------
FROM node:${NODE_VERSION}-alpine${ALPINE_VERSION} AS builder
WORKDIR /app
RUN corepack enable

COPY --from=deps /app/node_modules ./node_modules
COPY --from=deps /app/apps/web/node_modules ./apps/web/node_modules
COPY --from=deps /app/sdks/typescript/node_modules ./sdks/typescript/node_modules

# Bring in the manifests + workspace pieces the build needs. Keep the
# COPY list explicit so unrelated repo changes don't bust the cache.
COPY package.json pnpm-workspace.yaml pnpm-lock.yaml tsconfig.base.json ./
COPY apps/web ./apps/web
COPY sdks/typescript ./sdks/typescript

# SKIP_ENV_VALIDATION lets `next build` run without a populated .env;
# NEXT_PUBLIC_TL_SERVER_URL is inlined at runtime when the runtime stage
# is started (compose injects it). Build-time inlining of that var would
# bake in a hardcoded URL — we deliberately defer.
ENV NEXT_TELEMETRY_DISABLED=1 \
    SKIP_ENV_VALIDATION=true
RUN pnpm --filter web exec next build

# -----------------------------------------------------------------------------
FROM node:${NODE_VERSION}-alpine${ALPINE_VERSION} AS runtime
WORKDIR /app

# Run as a non-root user. The node image ships `node:node` (uid 1000).
USER node

ENV NODE_ENV=production \
    PORT=3000 \
    HOSTNAME=0.0.0.0 \
    NEXT_TELEMETRY_DISABLED=1

# Standalone bundle layout (with outputFileTracingRoot pointed at the
# monorepo root):
#   .next/standalone/                  — entry point (server.js) + traced node_modules
#   .next/standalone/apps/web/.next/   — required runtime chunks (we COPY static into here)
COPY --from=builder --chown=node:node /app/apps/web/.next/standalone ./
COPY --from=builder --chown=node:node /app/apps/web/.next/static ./apps/web/.next/static

EXPOSE 3000

# Healthcheck against `/` — the page renders even before the server is
# reachable, which is what compose wants for ordering.
HEALTHCHECK --interval=10s --timeout=3s --start-period=15s --retries=5 \
    CMD wget -qO- http://localhost:3000/ >/dev/null 2>&1 || exit 1

CMD ["node", "apps/web/server.js"]
