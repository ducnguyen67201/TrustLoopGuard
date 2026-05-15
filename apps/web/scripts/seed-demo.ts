import crypto from 'node:crypto';
import { drizzle } from 'drizzle-orm/postgres-js';
import postgres from 'postgres';

import { users } from '../lib/db/schema/auth';
import {
  guardrailDecisions,
  knowledgeSources,
  organizationMembers,
  organizations,
  workspaceAgents,
  workspaceApiKeys,
  workspaceMembers,
  workspacePolicies,
  workspaces,
  workspaceSettings,
} from '../lib/db/schema/workspace';

const DEMO_EMAIL = 'hoalu2112@gmail.com';
const USER_NAME = 'Hoalu Demo';
const ORG_ID = 'org_trustloop_demo';
const WORKSPACE_ID = 'ws_trustloop_demo';

const databaseUrl = process.env['DATABASE_URL'];
if (!databaseUrl) {
  throw new Error('DATABASE_URL is required to seed dashboard demo data.');
}

const client = postgres(databaseUrl, { max: 1 });
const db = drizzle(client);

async function main() {
  const [user] = await db
    .insert(users)
    .values({
      id: 'usr_hoalu2112',
      name: USER_NAME,
      email: DEMO_EMAIL,
    })
    .onConflictDoUpdate({
      target: users.email,
      set: {
        name: USER_NAME,
      },
    })
    .returning({ id: users.id });

  if (!user) {
    throw new Error(`Could not upsert demo user ${DEMO_EMAIL}`);
  }
  const userId = user.id;

  await db
    .insert(organizations)
    .values({
      id: ORG_ID,
      name: 'Trustloop Demo',
      slug: 'trustloop-demo',
    })
    .onConflictDoUpdate({
      target: organizations.id,
      set: {
        name: 'Trustloop Demo',
        slug: 'trustloop-demo',
        updatedAt: new Date(),
      },
    });

  await db
    .insert(organizationMembers)
    .values({
      organizationId: ORG_ID,
      userId,
      role: 'owner',
    })
    .onConflictDoUpdate({
      target: [organizationMembers.organizationId, organizationMembers.userId],
      set: {
        role: 'owner',
      },
    });

  await db
    .insert(workspaces)
    .values({
      id: WORKSPACE_ID,
      organizationId: ORG_ID,
      name: 'Trustloop Demo',
      slug: 'trustloop-demo',
      description: 'Demo workspace for managing TrustLoopGuard policies, agents, keys, and sources.',
    })
    .onConflictDoUpdate({
      target: workspaces.id,
      set: {
        name: 'Trustloop Demo',
        slug: 'trustloop-demo',
        description: 'Demo workspace for managing TrustLoopGuard policies, agents, keys, and sources.',
        updatedAt: new Date(),
        deletedAt: null,
      },
    });

  await db
    .insert(workspaceMembers)
    .values({
      workspaceId: WORKSPACE_ID,
      userId,
      role: 'owner',
    })
    .onConflictDoUpdate({
      target: [workspaceMembers.workspaceId, workspaceMembers.userId],
      set: {
        role: 'owner',
      },
    });

  await db
    .insert(workspaceSettings)
    .values({
      workspaceId: WORKSPACE_ID,
      defaultAction: 'allow',
      escalationWebhookUrl: 'https://hooks.example.com/trustloop-demo',
      telemetryEnabled: true,
      retentionDays: '30',
      config: {
        modelRouting: 'default',
        runtimeMode: 'single-config',
      },
    })
    .onConflictDoUpdate({
      target: workspaceSettings.workspaceId,
      set: {
        defaultAction: 'allow',
        escalationWebhookUrl: 'https://hooks.example.com/trustloop-demo',
        telemetryEnabled: true,
        retentionDays: '30',
        config: {
          modelRouting: 'default',
          runtimeMode: 'single-config',
        },
        updatedAt: new Date(),
      },
    });

  await seedAgents();
  await seedPolicies();
  await seedKnowledgeSources();
  await seedApiKeys(userId);
  await seedDecisions();

  console.log(`Seeded Trustloop Demo workspace for ${DEMO_EMAIL}`);
}

async function seedAgents() {
  await db
    .insert(workspaceAgents)
    .values([
      {
        id: 'agt_demo_support',
        workspaceId: WORKSPACE_ID,
        name: 'Support bot',
        scope: 'Billing, account, and product support',
        status: 'ready',
        systemPrompt:
          'Answer product and billing questions. Do not promise refunds, medical outcomes, legal outcomes, or unapproved discounts.',
      },
      {
        id: 'agt_demo_billing',
        workspaceId: WORKSPACE_ID,
        name: 'Billing triage',
        scope: 'Invoices, plan changes, and refund routing',
        status: 'needs_review',
        systemPrompt:
          'Triage billing issues and escalate refund exceptions to a human teammate.',
      },
      {
        id: 'agt_demo_sales',
        workspaceId: WORKSPACE_ID,
        name: 'Sales assistant',
        scope: 'Pre-sales qualification and pricing handoff',
        status: 'ready',
        systemPrompt:
          'Answer approved pricing questions and route custom contract requests to sales.',
      },
    ])
    .onConflictDoNothing();
}

async function seedPolicies() {
  await db
    .insert(workspacePolicies)
    .values([
      {
        id: 'pol_demo_refund',
        workspaceId: WORKSPACE_ID,
        agentId: 'agt_demo_support',
        policyKey: 'refund-guarantee',
        description: 'Block promises that guarantee refunds without approved policy context.',
        severity: 'high',
        action: 'block',
        enabled: true,
        sourceYaml: yamlPolicy('refund-guarantee', 'guaranteed refund', 'block'),
      },
      {
        id: 'pol_demo_pii',
        workspaceId: WORKSPACE_ID,
        agentId: 'agt_demo_support',
        policyKey: 'pii-leak',
        description: 'Escalate replies that include sensitive personal or payment data.',
        severity: 'critical',
        action: 'escalate',
        enabled: true,
        sourceYaml: yamlPolicy('pii-leak', 'credit card', 'escalate'),
      },
      {
        id: 'pol_demo_medical',
        workspaceId: WORKSPACE_ID,
        agentId: 'agt_demo_billing',
        policyKey: 'medical-advice',
        description: 'Escalate medical or health advice attempts to a human reviewer.',
        severity: 'critical',
        action: 'escalate',
        enabled: true,
        sourceYaml: yamlPolicy('medical-advice', 'medical advice', 'escalate'),
      },
      {
        id: 'pol_demo_discount',
        workspaceId: WORKSPACE_ID,
        agentId: 'agt_demo_sales',
        policyKey: 'unapproved-discount',
        description: 'Block discount commitments that are not in approved pricing docs.',
        severity: 'medium',
        action: 'block',
        enabled: false,
        sourceYaml: yamlPolicy('unapproved-discount', 'special discount', 'block'),
      },
      {
        id: 'pol_demo_tone',
        workspaceId: WORKSPACE_ID,
        agentId: 'agt_demo_support',
        policyKey: 'tone-softener',
        description: 'Rewrite dismissive support language into a calmer response.',
        severity: 'low',
        action: 'rewrite',
        enabled: true,
        sourceYaml: yamlPolicy('tone-softener', 'not our problem', 'rewrite'),
      },
    ])
    .onConflictDoNothing();
}

async function seedKnowledgeSources() {
  await db
    .insert(knowledgeSources)
    .values([
      {
        id: 'src_demo_refunds',
        workspaceId: WORKSPACE_ID,
        title: 'Refund policy',
        kind: 'url',
        location: 'https://example.com/help/refunds',
        status: 'ready',
        metadata: { owner: 'support' },
        lastIndexedAt: minutesAgo(45),
      },
      {
        id: 'src_demo_pricing',
        workspaceId: WORKSPACE_ID,
        title: 'Pricing exceptions',
        kind: 'note',
        location: 'Manual workspace note',
        status: 'ready',
        metadata: { owner: 'sales' },
        lastIndexedAt: hoursAgo(5),
      },
      {
        id: 'src_demo_security',
        workspaceId: WORKSPACE_ID,
        title: 'Security FAQ',
        kind: 'file',
        location: 'security-faq.pdf',
        status: 'indexing',
        metadata: { owner: 'security' },
        lastIndexedAt: null,
      },
    ])
    .onConflictDoNothing();
}

async function seedApiKeys(userId: string) {
  await db
    .insert(workspaceApiKeys)
    .values([
      {
        id: 'key_demo_prod',
        workspaceId: WORKSPACE_ID,
        name: 'Production runtime',
        keyPrefix: 'tlg_live_demo',
        keyHash: hashKey('tlg_live_demo_secret'),
        status: 'active',
        createdByUserId: userId,
        lastUsedAt: minutesAgo(12),
      },
      {
        id: 'key_demo_ci',
        workspaceId: WORKSPACE_ID,
        name: 'CI policy tests',
        keyPrefix: 'tlg_ci_demo',
        keyHash: hashKey('tlg_ci_demo_secret'),
        status: 'active',
        createdByUserId: userId,
        lastUsedAt: hoursAgo(18),
      },
    ])
    .onConflictDoNothing();
}

async function seedDecisions() {
  await db
    .insert(guardrailDecisions)
    .values([
      {
        id: 'dec_demo_001',
        workspaceId: WORKSPACE_ID,
        agentId: 'agt_demo_support',
        policyId: 'pol_demo_refund',
        traceId: 'trc_demo_001',
        verdict: 'block',
        latencyMs: '132',
        createdAt: minutesAgo(2),
      },
      {
        id: 'dec_demo_002',
        workspaceId: WORKSPACE_ID,
        agentId: 'agt_demo_billing',
        policyId: 'pol_demo_medical',
        traceId: 'trc_demo_002',
        verdict: 'escalate',
        latencyMs: '218',
        createdAt: minutesAgo(8),
      },
      {
        id: 'dec_demo_003',
        workspaceId: WORKSPACE_ID,
        agentId: 'agt_demo_support',
        policyId: 'pol_demo_pii',
        traceId: 'trc_demo_003',
        verdict: 'escalate',
        latencyMs: '177',
        createdAt: minutesAgo(14),
      },
      {
        id: 'dec_demo_004',
        workspaceId: WORKSPACE_ID,
        agentId: 'agt_demo_support',
        policyId: 'pol_demo_tone',
        traceId: 'trc_demo_004',
        verdict: 'rewrite',
        latencyMs: '166',
        createdAt: minutesAgo(21),
      },
      {
        id: 'dec_demo_005',
        workspaceId: WORKSPACE_ID,
        agentId: 'agt_demo_sales',
        policyId: null,
        traceId: 'trc_demo_005',
        verdict: 'allow',
        latencyMs: '84',
        createdAt: minutesAgo(35),
      },
    ])
    .onConflictDoNothing();
}

function yamlPolicy(id: string, literal: string, action: string): string {
  return `id: ${id}
match:
  literal: "${literal}"
action: ${action}
`;
}

function hashKey(value: string): string {
  return crypto.createHash('sha256').update(value).digest('hex');
}

function minutesAgo(minutes: number): Date {
  return new Date(Date.now() - minutes * 60_000);
}

function hoursAgo(hours: number): Date {
  return new Date(Date.now() - hours * 60 * 60_000);
}

main()
  .catch((error) => {
    console.error(error);
    process.exitCode = 1;
  })
  .finally(async () => {
    await client.end();
  });
