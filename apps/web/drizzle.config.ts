import { defineConfig } from 'drizzle-kit';

export default defineConfig({
  schema: './lib/db/schema/auth.ts',
  out: './lib/db/migrations',
  dialect: 'postgresql',
  dbCredentials: {
    url: process.env['DATABASE_URL'] ?? '',
  },
  tablesFilter: ['auth_*'],
});
