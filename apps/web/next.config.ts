import path from 'node:path';
import type { NextConfig } from 'next';

// Deploy stamp: changes on every build. Railway/Vercel expose the commit SHA;
// locally it falls back to a per-build timestamp. Baked into the client bundle
// (env) and served fresh by /api/version so an open tab can detect a new deploy.
// ponytail: single build stamp, no rolling-deploy dedup — fine for one instance.
const buildId =
  process.env['RAILWAY_GIT_COMMIT_SHA'] ??
  process.env['VERCEL_GIT_COMMIT_SHA'] ??
  `dev-${Date.now()}`;

const config: NextConfig = {
  reactStrictMode: true,
  env: { NEXT_PUBLIC_BUILD_ID: buildId },
  generateBuildId: () => buildId,
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
