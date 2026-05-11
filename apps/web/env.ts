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
    DATABASE_URL: z.string().url(),
    AUTH_SECRET: z.string().min(32),
    AUTH_ALLOW_SIGNUP: z
      .enum(['true', 'false'])
      .default('true')
      .transform((v) => v === 'true'),
    AUTH_GOOGLE_ID: z.string().min(1).optional(),
    AUTH_GOOGLE_SECRET: z.string().min(1).optional(),
    AUTH_TRUST_HOST: z
      .enum(['true', 'false'])
      .default('false')
      .transform((v) => v === 'true'),
    TL_ADMIN_KEY: z.string().min(1).optional(),
    TL_SERVER_INTERNAL_URL: z.string().url().default('http://localhost:8080'),
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
    DATABASE_URL: process.env['DATABASE_URL'],
    AUTH_SECRET: process.env['AUTH_SECRET'],
    AUTH_ALLOW_SIGNUP: process.env['AUTH_ALLOW_SIGNUP'],
    AUTH_GOOGLE_ID: process.env['AUTH_GOOGLE_ID'],
    AUTH_GOOGLE_SECRET: process.env['AUTH_GOOGLE_SECRET'],
    AUTH_TRUST_HOST: process.env['AUTH_TRUST_HOST'],
    TL_ADMIN_KEY: process.env['TL_ADMIN_KEY'],
    TL_SERVER_INTERNAL_URL: process.env['TL_SERVER_INTERNAL_URL'],
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

if (
  (env.AUTH_GOOGLE_ID && !env.AUTH_GOOGLE_SECRET) ||
  (!env.AUTH_GOOGLE_ID && env.AUTH_GOOGLE_SECRET)
) {
  throw new Error(
    'AUTH_GOOGLE_ID and AUTH_GOOGLE_SECRET must be set together (or both unset).',
  );
}
