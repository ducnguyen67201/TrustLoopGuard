import { defineConfig } from 'drizzle-kit';

export default defineConfig({
  schema: ['./lib/db/schema/auth.ts', './lib/db/schema/workspace.ts'],
  out: './lib/db/migrations',
  dialect: 'postgresql',
  dbCredentials: {
    url: process.env['DATABASE_URL'] ?? '',
  },
  tablesFilter: ['auth_*', 'organizations', 'organization_*', 'workspaces', 'workspace_*', 'knowledge_*', 'guardrail_*'],
});
