# syntax=docker/dockerfile:1.7

ARG NODE_VERSION=22
ARG ALPINE_VERSION=3.20

FROM node:${NODE_VERSION}-alpine${ALPINE_VERSION} AS deps
WORKDIR /app
RUN corepack enable

COPY package.json pnpm-workspace.yaml pnpm-lock.yaml tsconfig.base.json ./
COPY apps/marketing/package.json ./apps/marketing/

RUN PNPM_HOME=/pnpm pnpm config set store-dir /pnpm/store \
 && pnpm install --frozen-lockfile --ignore-scripts

FROM node:${NODE_VERSION}-alpine${ALPINE_VERSION} AS builder
WORKDIR /app
RUN corepack enable

ARG NEXT_PUBLIC_BOOK_MEETING_URL

COPY --from=deps /app/node_modules ./node_modules
COPY --from=deps /app/apps/marketing/node_modules ./apps/marketing/node_modules

COPY package.json pnpm-workspace.yaml pnpm-lock.yaml tsconfig.base.json ./
COPY apps/marketing ./apps/marketing

ENV NEXT_TELEMETRY_DISABLED=1 \
    NEXT_PUBLIC_BOOK_MEETING_URL=${NEXT_PUBLIC_BOOK_MEETING_URL}
RUN pnpm --filter marketing build

FROM node:${NODE_VERSION}-alpine${ALPINE_VERSION} AS runtime
WORKDIR /app

USER node

ENV NODE_ENV=production \
    PORT=3000 \
    HOSTNAME=0.0.0.0 \
    NEXT_TELEMETRY_DISABLED=1

COPY --from=builder --chown=node:node /app/apps/marketing/.next/standalone ./
COPY --from=builder --chown=node:node /app/apps/marketing/.next/static ./apps/marketing/.next/static
COPY --from=builder --chown=node:node /app/apps/marketing/public ./apps/marketing/public

EXPOSE 3000

HEALTHCHECK --interval=10s --timeout=3s --start-period=15s --retries=5 \
    CMD wget -qO- http://localhost:${PORT}/ >/dev/null 2>&1 || exit 1

CMD ["node", "apps/marketing/server.js"]
