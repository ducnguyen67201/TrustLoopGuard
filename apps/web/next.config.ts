import path from 'node:path';
import type { NextConfig } from 'next';

const config: NextConfig = {
  reactStrictMode: true,
  // Consume @trustloopguard/sdk source-first via the workspace exports map.
  // Next compiles it through SWC instead of expecting a prebuilt dist/.
  transpilePackages: ['@trustloopguard/sdk'],
  typescript: {
    ignoreBuildErrors: false,
  },
  experimental: {
    serverActions: {
      bodySizeLimit: '10mb',
    },
  },
  // Emit a self-contained server bundle at .next/standalone with only the
  // node_modules Next traced as actually used. The Docker runtime stage
  // copies that and runs `node server.js` — no `pnpm install` at runtime,
  // ~150 MB final image instead of ~1 GB.
  output: 'standalone',
  // Trace files from the monorepo root so the standalone bundle picks
  // up the workspace-symlinked @trustloopguard/sdk and its hoisted
  // node_modules. Without this, Next defaults to the app dir and ships
  // a broken bundle in pnpm/yarn workspaces.
  outputFileTracingRoot: path.resolve(import.meta.dirname, '../../'),
};

export default config;
