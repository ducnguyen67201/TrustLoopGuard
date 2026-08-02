# syntax=docker/dockerfile:1.7
#
# Featherlane AI docs (Next.js + Fumadocs) multi-stage image.

ARG NODE_VERSION=22
ARG ALPINE_VERSION=3.20

FROM node:${NODE_VERSION}-alpine${ALPINE_VERSION} AS deps
WORKDIR /app
RUN corepack enable

COPY package.json pnpm-workspace.yaml pnpm-lock.yaml tsconfig.base.json ./
COPY apps/docs/package.json ./apps/docs/

RUN PNPM_HOME=/pnpm pnpm config set store-dir /pnpm/store \
 && pnpm install --frozen-lockfile --ignore-scripts

FROM node:${NODE_VERSION}-alpine${ALPINE_VERSION} AS builder
WORKDIR /app
RUN corepack enable

COPY --from=deps /app/node_modules ./node_modules
COPY --from=deps /app/apps/docs/node_modules ./apps/docs/node_modules

COPY package.json pnpm-workspace.yaml pnpm-lock.yaml tsconfig.base.json ./
COPY docs/openapi.yaml ./docs/openapi.yaml
COPY apps/docs ./apps/docs

ENV NEXT_TELEMETRY_DISABLED=1
RUN pnpm --filter docs exec fumadocs-mdx \
 && pnpm --filter docs exec next build

FROM node:${NODE_VERSION}-alpine${ALPINE_VERSION} AS runtime
WORKDIR /app

USER node

ENV NODE_ENV=production \
    PORT=3001 \
    HOSTNAME=0.0.0.0 \
    NEXT_TELEMETRY_DISABLED=1

COPY --from=builder --chown=node:node /app/apps/docs/.next/standalone ./
COPY --from=builder --chown=node:node /app/apps/docs/.next/static ./apps/docs/.next/static

EXPOSE 3001

HEALTHCHECK --interval=10s --timeout=3s --start-period=15s --retries=5 \
    CMD wget -qO- http://localhost:3001/docs >/dev/null 2>&1 || exit 1

CMD ["node", "apps/docs/server.js"]
