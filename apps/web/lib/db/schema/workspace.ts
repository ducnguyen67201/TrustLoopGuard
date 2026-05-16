import {
  boolean,
  customType,
  index,
  integer,
  jsonb,
  pgEnum,
  pgTable,
  primaryKey,
  text,
  timestamp,
  uniqueIndex,
  uuid,
} from 'drizzle-orm/pg-core';
import { Buffer } from 'node:buffer';

import { users } from './auth';

const bytea = customType<{ data: Buffer; driverData: Buffer | Uint8Array }>({
  dataType() {
    return 'bytea';
  },
  toDriver(value) {
    return value;
  },
  fromDriver(value) {
    return Buffer.from(value);
  },
});

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

export type RuntimeAgentProfile = {
  agent_id: string;
  display_name?: string;
  system_prompt?: string;
  scope?: {
    in_scope?: string[];
    out_of_scope?: string[];
  };
  authority?: {
    can_promise?: string[];
    cannot_promise?: string[];
  };
  tone?: {
    target?: string;
    forbidden?: string[];
  };
  knowledge_sources?: unknown[];
  escalation_triggers?: string[];
};

export type RuntimePolicyDocument = {
  id: string;
  description?: string | null;
  match?: Record<string, unknown>;
  action?: string;
  rewrite?: string | null;
  severity?: string;
  owner_agent_id?: string | null;
};

export type RuntimeDecisionPayload = {
  trace_id?: string;
  verdict?: string;
  reason?: string;
  triggered_policies?: Array<{ id?: string; severity?: string; reason?: string }>;
  safe_output?: string | null;
  latency_ms?: number;
  agent_id?: string;
};

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

export const knowledgeSourceFiles = pgTable('knowledge_source_files', {
  knowledgeSourceId: text('knowledge_source_id')
    .primaryKey()
    .references(() => knowledgeSources.id, { onDelete: 'cascade' }),
  fileName: text('file_name').notNull(),
  mediaType: text('media_type').notNull(),
  byteSize: integer('byte_size').notNull(),
  checksumSha256: text('checksum_sha256').notNull(),
  data: bytea('data').notNull(),
  createdAt: timestamp('created_at').notNull().defaultNow(),
  updatedAt: timestamp('updated_at').notNull().defaultNow(),
});

export const runtimeAgents = pgTable(
  'agents',
  {
    workspaceId: text('workspace_id').notNull().default('default'),
    id: text('id').notNull(),
    profileYaml: text('profile_yaml').notNull(),
    parsedProfile: jsonb('parsed_profile').$type<RuntimeAgentProfile>().notNull(),
    createdAt: timestamp('created_at').notNull().defaultNow(),
    updatedAt: timestamp('updated_at').notNull().defaultNow(),
    deletedAt: timestamp('deleted_at'),
  },
  (agent) => ({
    pk: primaryKey({ columns: [agent.workspaceId, agent.id] }),
    activeIdx: index('agents_active_idx').on(agent.workspaceId, agent.id),
  }),
);

export const runtimePolicies = pgTable(
  'policies',
  {
    workspaceId: text('workspace_id').notNull().default('default'),
    id: text('id').notNull(),
    policyYaml: text('policy_yaml').notNull(),
    parsedPolicy: jsonb('parsed_policy').$type<RuntimePolicyDocument>().notNull(),
    enabled: boolean('enabled').notNull().default(true),
    ownerAgentId: text('owner_agent_id'),
    createdAt: timestamp('created_at').notNull().defaultNow(),
    updatedAt: timestamp('updated_at').notNull().defaultNow(),
    deletedAt: timestamp('deleted_at'),
  },
  (policy) => ({
    pk: primaryKey({ columns: [policy.workspaceId, policy.id] }),
    activeIdx: index('policies_active_idx').on(policy.workspaceId, policy.id),
    enabledIdx: index('policies_enabled_idx').on(policy.workspaceId, policy.enabled, policy.id),
    ownerAgentIdx: index('policies_owner_agent_idx').on(policy.workspaceId, policy.ownerAgentId),
  }),
);

export const runtimeTraces = pgTable(
  'traces',
  {
    workspaceId: text('workspace_id').notNull().default('default'),
    traceId: uuid('trace_id').notNull(),
    domain: text('domain').notNull(),
    decision: text('decision').notNull(),
    elapsedMs: integer('elapsed_ms').notNull(),
    payload: jsonb('payload').$type<RuntimeDecisionPayload>().notNull(),
    createdAt: timestamp('created_at').notNull().defaultNow(),
  },
  (trace) => ({
    pk: primaryKey({ columns: [trace.traceId, trace.createdAt] }),
    workspaceDecisionIdx: index('traces_workspace_decision_idx').on(
      trace.workspaceId,
      trace.decision,
      trace.createdAt,
    ),
    workspaceDomainIdx: index('traces_workspace_domain_idx').on(
      trace.workspaceId,
      trace.domain,
      trace.createdAt,
    ),
  }),
);
