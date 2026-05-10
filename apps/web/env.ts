// Single source of truth for environment variables in apps/web.
//
// `createEnv` validates env at build time *and* at runtime, fails fast
// with a clear error if anything is missing or malformed, and exports
// a typed `env` object the rest of the app imports instead of touching
// `process.env` directly.
//
// Splitting `server` and `client` enforces the boundary: client-side
// code reading `env.SOME_SERVER_SECRET` is a compile error.

import { createEnv } from '@t3-oss/env-nextjs';
import { z } from 'zod';

export const env = createEnv({
  server: {
    NODE_ENV: z.enum(['development', 'test', 'production']).default('development'),
    // Origin of the deployed docs site (apps/docs). Used by next.config
    // to rewrite /docs/:path*, /llms.txt, etc. so the marketing site
    // and the docs share one public origin (e.g. trustloopguard.dev).
    DOCS_ORIGIN: z.string().url().default('http://localhost:3001'),
  },
  client: {
    NEXT_PUBLIC_TL_SERVER_URL: z
      .string()
      .url()
      .default('http://localhost:8080'),
  },
  // Next inlines NEXT_PUBLIC_* at build time, so we cannot destructure
  // process.env (the build-time substitution only works on direct
  // member access). Listing each var here makes the inlining work.
  runtimeEnv: {
    NODE_ENV: process.env['NODE_ENV'],
    DOCS_ORIGIN: process.env['DOCS_ORIGIN'],
    NEXT_PUBLIC_TL_SERVER_URL: process.env['NEXT_PUBLIC_TL_SERVER_URL'],
  },
  // Treat empty strings as undefined so a blank .env entry falls back
  // to the schema default instead of failing the URL validator.
  emptyStringAsUndefined: true,
  // Skip validation in lint / type-check steps where envs aren't loaded.
  skipValidation:
    process.env['SKIP_ENV_VALIDATION'] === 'true' ||
    process.env['npm_lifecycle_event'] === 'lint',
});
