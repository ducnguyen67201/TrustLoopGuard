import 'server-only';

import { drizzle } from 'drizzle-orm/postgres-js';
import postgres from 'postgres';

import { env } from '@/env';
import * as authSchema from './schema/auth';

const queryClient = postgres(env.DATABASE_URL, { max: 5 });

export const db = drizzle(queryClient, {
  schema: { ...authSchema },
});

export type DB = typeof db;
