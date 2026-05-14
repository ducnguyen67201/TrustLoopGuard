import 'server-only';

import { drizzle } from 'drizzle-orm/postgres-js';
import postgres from 'postgres';

import { env } from '@/env';
import * as authSchema from './schema/auth';

function createDb() {
  if (!env.DATABASE_URL) {
    throw new Error(
      'DATABASE_URL is not set. Configure it (Doppler or .env.local) to use auth/db features.',
    );
  }
  const queryClient = postgres(env.DATABASE_URL, { max: 5 });
  return drizzle(queryClient, { schema: { ...authSchema } });
}

let cached: ReturnType<typeof createDb> | undefined;
export const db = new Proxy({} as ReturnType<typeof createDb>, {
  get(_target, prop) {
    cached ??= createDb();
    return Reflect.get(cached, prop);
  },
});

export type DB = ReturnType<typeof createDb>;
