import {
  boolean,
  index,
  jsonb,
  pgEnum,
  pgTable,
  primaryKey,
  text,
  timestamp,
  uniqueIndex,
} from 'drizzle-orm/pg-core';

import { users } from './auth';

export const organizationRole = pgEnum('organization_role', ['owner', 'admin', 'member']);
export const workspaceRole = pgEnum('workspace_role', ['owner', 'admin', 'editor', 'viewer']);
export const inviteStatus = pgEnum('invite_status', ['pending', 'accepted', 'revoked', 'expired']);
export const apiKeyStatus = pgEnum('api_key_status', ['active', 'revoked']);
export const knowledgeSourceKind = pgEnum('knowledge_source_kind', ['url', 'file', 'note']);
export const knowledgeSourceStatus = pgEnum('knowledge_source_status', [
  'draft',
  'indexing',
  'ready',
  'failed',
]);
export const workspacePolicySeverity = pgEnum('workspace_policy_severity', [
  'low',
  'medium',
  'high',
  'critical',
]);
export const workspacePolicyAction = pgEnum('workspace_policy_action', [
  'allow',
  'block',
  'rewrite',
  'escalate',
]);
export const workspaceAgentStatus = pgEnum('workspace_agent_status', [
  'ready',
  'needs_review',
  'draft',
]);
export const guardrailVerdict = pgEnum('guardrail_verdict', [
  'allow',
  'block',
  'rewrite',
  'escalate',
]);

export const organizations = pgTable('organizations', {
  id: text('id')
    .primaryKey()
    .$defaultFn(() => crypto.randomUUID()),
  name: text('name').notNull(),
  slug: text('slug').notNull().unique(),
  createdAt: timestamp('created_at').notNull().defaultNow(),
  updatedAt: timestamp('updated_at').notNull().defaultNow(),
});

export const organizationMembers = pgTable(
  'organization_members',
  {
    organizationId: text('organization_id')
      .notNull()
      .references(() => organizations.id, { onDelete: 'cascade' }),
    userId: text('user_id')
      .notNull()
      .references(() => users.id, { onDelete: 'cascade' }),
    role: organizationRole('role').notNull().default('member'),
    createdAt: timestamp('created_at').notNull().defaultNow(),
  },
  (member) => ({
    pk: primaryKey({ columns: [member.organizationId, member.userId] }),
    userIdx: index('organization_members_user_idx').on(member.userId),
  }),
);

export const workspaces = pgTable(
  'workspaces',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => crypto.randomUUID()),
    organizationId: text('organization_id')
      .notNull()
      .references(() => organizations.id, { onDelete: 'cascade' }),
    name: text('name').notNull(),
    slug: text('slug').notNull(),
    description: text('description'),
    createdAt: timestamp('created_at').notNull().defaultNow(),
    updatedAt: timestamp('updated_at').notNull().defaultNow(),
    deletedAt: timestamp('deleted_at'),
  },
  (workspace) => ({
    organizationSlugIdx: uniqueIndex('workspaces_organization_slug_idx').on(
      workspace.organizationId,
      workspace.slug,
    ),
    activeIdx: index('workspaces_active_idx').on(workspace.organizationId, workspace.deletedAt),
  }),
);

export const workspaceMembers = pgTable(
  'workspace_members',
  {
    workspaceId: text('workspace_id')
      .notNull()
      .references(() => workspaces.id, { onDelete: 'cascade' }),
    userId: text('user_id')
      .notNull()
      .references(() => users.id, { onDelete: 'cascade' }),
    role: workspaceRole('role').notNull().default('viewer'),
    createdAt: timestamp('created_at').notNull().defaultNow(),
  },
  (member) => ({
    pk: primaryKey({ columns: [member.workspaceId, member.userId] }),
    userIdx: index('workspace_members_user_idx').on(member.userId),
  }),
);

export const workspaceInvites = pgTable(
  'workspace_invites',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => crypto.randomUUID()),
    workspaceId: text('workspace_id')
      .notNull()
      .references(() => workspaces.id, { onDelete: 'cascade' }),
    email: text('email').notNull(),
    role: workspaceRole('role').notNull().default('viewer'),
    status: inviteStatus('status').notNull().default('pending'),
    invitedByUserId: text('invited_by_user_id').references(() => users.id, { onDelete: 'set null' }),
    createdAt: timestamp('created_at').notNull().defaultNow(),
    expiresAt: timestamp('expires_at').notNull(),
  },
  (invite) => ({
    pendingEmailIdx: index('workspace_invites_pending_email_idx').on(
      invite.workspaceId,
      invite.email,
      invite.status,
    ),
  }),
);

export const workspaceSettings = pgTable('workspace_settings', {
  workspaceId: text('workspace_id')
    .primaryKey()
    .references(() => workspaces.id, { onDelete: 'cascade' }),
  defaultAction: text('default_action').notNull().default('allow'),
  escalationWebhookUrl: text('escalation_webhook_url'),
  telemetryEnabled: boolean('telemetry_enabled').notNull().default(true),
  retentionDays: text('retention_days').notNull().default('30'),
  config: jsonb('config').$type<Record<string, unknown>>().notNull().default({}),
  updatedAt: timestamp('updated_at').notNull().defaultNow(),
});

export const workspaceApiKeys = pgTable(
  'workspace_api_keys',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => crypto.randomUUID()),
    workspaceId: text('workspace_id')
      .notNull()
      .references(() => workspaces.id, { onDelete: 'cascade' }),
    name: text('name').notNull(),
    keyPrefix: text('key_prefix').notNull(),
    keyHash: text('key_hash').notNull().unique(),
    status: apiKeyStatus('status').notNull().default('active'),
    createdByUserId: text('created_by_user_id').references(() => users.id, { onDelete: 'set null' }),
    createdAt: timestamp('created_at').notNull().defaultNow(),
    lastUsedAt: timestamp('last_used_at'),
    revokedAt: timestamp('revoked_at'),
  },
  (apiKey) => ({
    workspaceStatusIdx: index('workspace_api_keys_status_idx').on(apiKey.workspaceId, apiKey.status),
    prefixIdx: index('workspace_api_keys_prefix_idx').on(apiKey.keyPrefix),
  }),
);

export const knowledgeSources = pgTable(
  'knowledge_sources',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => crypto.randomUUID()),
    workspaceId: text('workspace_id')
      .notNull()
      .references(() => workspaces.id, { onDelete: 'cascade' }),
    title: text('title').notNull(),
    kind: knowledgeSourceKind('kind').notNull(),
    location: text('location'),
    status: knowledgeSourceStatus('status').notNull().default('draft'),
    metadata: jsonb('metadata').$type<Record<string, unknown>>().notNull().default({}),
    createdAt: timestamp('created_at').notNull().defaultNow(),
    updatedAt: timestamp('updated_at').notNull().defaultNow(),
    lastIndexedAt: timestamp('last_indexed_at'),
    deletedAt: timestamp('deleted_at'),
  },
  (source) => ({
    workspaceStatusIdx: index('knowledge_sources_workspace_status_idx').on(
      source.workspaceId,
      source.status,
    ),
  }),
);

export const workspaceAgents = pgTable(
  'workspace_agents',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => crypto.randomUUID()),
    workspaceId: text('workspace_id')
      .notNull()
      .references(() => workspaces.id, { onDelete: 'cascade' }),
    name: text('name').notNull(),
    scope: text('scope').notNull(),
    status: workspaceAgentStatus('status').notNull().default('draft'),
    systemPrompt: text('system_prompt'),
    createdAt: timestamp('created_at').notNull().defaultNow(),
    updatedAt: timestamp('updated_at').notNull().defaultNow(),
    deletedAt: timestamp('deleted_at'),
  },
  (agent) => ({
    workspaceIdx: index('workspace_agents_workspace_idx').on(agent.workspaceId, agent.deletedAt),
  }),
);

export const workspacePolicies = pgTable(
  'workspace_policies',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => crypto.randomUUID()),
    workspaceId: text('workspace_id')
      .notNull()
      .references(() => workspaces.id, { onDelete: 'cascade' }),
    agentId: text('agent_id').references(() => workspaceAgents.id, { onDelete: 'set null' }),
    policyKey: text('policy_key').notNull(),
    description: text('description').notNull(),
    severity: workspacePolicySeverity('severity').notNull().default('medium'),
    action: workspacePolicyAction('action').notNull().default('block'),
    enabled: boolean('enabled').notNull().default(false),
    sourceYaml: text('source_yaml'),
    createdAt: timestamp('created_at').notNull().defaultNow(),
    updatedAt: timestamp('updated_at').notNull().defaultNow(),
    deletedAt: timestamp('deleted_at'),
  },
  (policy) => ({
    workspaceKeyIdx: uniqueIndex('workspace_policies_workspace_key_idx').on(
      policy.workspaceId,
      policy.policyKey,
    ),
    workspaceEnabledIdx: index('workspace_policies_enabled_idx').on(
      policy.workspaceId,
      policy.enabled,
      policy.deletedAt,
    ),
  }),
);

export const guardrailDecisions = pgTable(
  'guardrail_decisions',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => crypto.randomUUID()),
    workspaceId: text('workspace_id')
      .notNull()
      .references(() => workspaces.id, { onDelete: 'cascade' }),
    agentId: text('agent_id').references(() => workspaceAgents.id, { onDelete: 'set null' }),
    policyId: text('policy_id').references(() => workspacePolicies.id, { onDelete: 'set null' }),
    traceId: text('trace_id').notNull(),
    verdict: guardrailVerdict('verdict').notNull(),
    latencyMs: text('latency_ms').notNull(),
    createdAt: timestamp('created_at').notNull().defaultNow(),
  },
  (decision) => ({
    workspaceCreatedIdx: index('guardrail_decisions_workspace_created_idx').on(
      decision.workspaceId,
      decision.createdAt,
    ),
  }),
);
