# syntax=docker/dockerfile:1.7

FROM node:22-alpine

WORKDIR /app
RUN corepack enable

COPY package.json pnpm-workspace.yaml pnpm-lock.yaml tsconfig.base.json ./
COPY demo/package.json ./demo/package.json
COPY sdks/typescript/package.json ./sdks/typescript/package.json

RUN PNPM_HOME=/pnpm pnpm config set store-dir /pnpm/store \
 && pnpm install --frozen-lockfile --ignore-scripts

COPY demo ./demo
COPY sdks/typescript ./sdks/typescript

ENV NODE_ENV=production \
    PORT=8080 \
    STRIPE_REFUND_AGENT_UI_HOST=0.0.0.0

USER node
EXPOSE 8080

CMD ["pnpm", "--filter", "@trustloopguard/demo", "stripe-refund-agent:ui"]
