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

type AppEnv = 'dev' | 'stage' | 'prod';
const appEnvSchema = z.enum(['development', 'staging', 'prod']);

const publicUrlOrPath = z.string().min(1).refine(
  (value) => value.startsWith('/') || URL.canParse(value),
  { message: 'Must be an absolute URL or root-relative path' },
);

export function checkEnv(): AppEnv {
  const appEnv = process.env['NEXT_PUBLIC_APP_ENV'] ?? process.env['APP_ENV'];

  if (appEnv === 'development') {
    return 'dev';
  }

  if (appEnv === 'staging' || process.env['VERCEL_ENV'] === 'preview') {
    return 'stage';
  }

  if (appEnv === 'prod' || process.env['VERCEL_ENV'] === 'production') {
    return 'prod';
  }

  return process.env['NODE_ENV'] === 'production' ? 'prod' : 'dev';
}

function defaultDocsUrl() {
  return checkEnv() === 'dev' ? 'http://localhost:3001/docs' : '/apps/doc';
}

export const env = createEnv({
  server: {
    NODE_ENV: z.enum(['development', 'test', 'production']).default('development'),
    AUTH_SECRET: z.string().min(32).optional(),
    AUTH_GITHUB_ID: z.string().optional(),
    AUTH_GITHUB_SECRET: z.string().optional(),
    AUTH_GOOGLE_ID: z.string().optional(),
    AUTH_GOOGLE_SECRET: z.string().optional(),
    TL_SERVER_URL: z.string().url().default('http://127.0.0.1:8080'),
  },
  client: {
    NEXT_PUBLIC_TL_SERVER_URL: z
      .string()
      .url()
      .default('http://localhost:8080'),
    NEXT_PUBLIC_APP_ENV: appEnvSchema.optional(),
    NEXT_PUBLIC_DOCS_URL: publicUrlOrPath.default(defaultDocsUrl()),
  },
  // Next inlines NEXT_PUBLIC_* at build time, so we cannot destructure
  // process.env (the build-time substitution only works on direct
  // member access). Listing each var here makes the inlining work.
  runtimeEnv: {
    NODE_ENV: process.env['NODE_ENV'],
    AUTH_SECRET: process.env['AUTH_SECRET'],
    AUTH_GITHUB_ID: process.env['AUTH_GITHUB_ID'],
    AUTH_GITHUB_SECRET: process.env['AUTH_GITHUB_SECRET'],
    AUTH_GOOGLE_ID: process.env['AUTH_GOOGLE_ID'],
    AUTH_GOOGLE_SECRET: process.env['AUTH_GOOGLE_SECRET'],
    TL_SERVER_URL: process.env['TL_SERVER_URL'],
    NEXT_PUBLIC_TL_SERVER_URL: process.env['NEXT_PUBLIC_TL_SERVER_URL'],
    NEXT_PUBLIC_APP_ENV: process.env['NEXT_PUBLIC_APP_ENV'],
    NEXT_PUBLIC_DOCS_URL: process.env['NEXT_PUBLIC_DOCS_URL'],
  },
  // Treat empty strings as undefined so a blank .env entry falls back
  // to the schema default instead of failing the URL validator.
  emptyStringAsUndefined: true,
  // Skip validation in lint / type-check steps where envs aren't loaded.
  skipValidation:
    process.env['SKIP_ENV_VALIDATION'] === 'true' ||
    process.env['npm_lifecycle_event'] === 'lint',
});
