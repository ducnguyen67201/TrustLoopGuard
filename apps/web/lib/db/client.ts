import 'server-only';

import { drizzle } from 'drizzle-orm/postgres-js';
import postgres from 'postgres';

import { env } from '@/env';
import * as authSchema from './schema/auth';
import * as workspaceSchema from './schema/workspace';

type DbClient = ReturnType<typeof createDb>;

const globalForDb = globalThis as typeof globalThis & {
  trustLoopDb?: DbClient;
};

function createDb() {
  if (!env.DATABASE_URL) {
    throw new Error(
      'DATABASE_URL is not set. Configure it (Doppler or .env.local) to use auth/db features.',
    );
  }
  const queryClient = postgres(env.DATABASE_URL, {
    idle_timeout: 20,
    max: 2,
  });
  return drizzle(queryClient, { schema: { ...authSchema, ...workspaceSchema } });
}

export function getDb(): DbClient {
  globalForDb.trustLoopDb ??= createDb();
  return globalForDb.trustLoopDb;
}

export const db = new Proxy({} as ReturnType<typeof createDb>, {
  get(_target, prop) {
    return Reflect.get(getDb(), prop);
  },
});

export type DB = ReturnType<typeof createDb>;
