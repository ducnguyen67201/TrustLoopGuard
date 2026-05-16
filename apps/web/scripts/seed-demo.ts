import crypto from 'node:crypto';
import { eq } from 'drizzle-orm';
import { drizzle } from 'drizzle-orm/postgres-js';
import postgres from 'postgres';

import { users } from '../lib/db/schema/auth';
import {
  knowledgeSources,
  organizationMembers,
  organizations,
  workspaceApiKeys,
  workspaceMembers,
  runtimeAgents,
  runtimePolicies,
  runtimeTraces,
  type RuntimeAgentProfile,
  type RuntimeDecisionPayload,
  type RuntimePolicyDocument,
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
  const agents = [
    {
      id: 'agt_demo_support',
      name: 'Support bot',
      scope: 'Billing, account, and product support',
      systemPrompt:
        'Answer product and billing questions. Do not promise refunds, medical outcomes, legal outcomes, or unapproved discounts.',
    },
    {
      id: 'agt_demo_billing',
      name: 'Billing triage',
      scope: 'Invoices, plan changes, and refund routing',
      systemPrompt: 'Triage billing issues and escalate refund exceptions to a human teammate.',
    },
    {
      id: 'agt_demo_sales',
      name: 'Sales assistant',
      scope: 'Pre-sales qualification and pricing handoff',
      systemPrompt: 'Answer approved pricing questions and route custom contract requests to sales.',
    },
  ];

  for (const agent of agents) {
    const parsedProfile = runtimeAgent(agent.id, agent.name, agent.scope, agent.systemPrompt);
    await db
      .insert(runtimeAgents)
      .values({
        workspaceId: WORKSPACE_ID,
        id: agent.id,
        profileYaml: agentYaml(parsedProfile),
        parsedProfile,
        updatedAt: new Date(),
        deletedAt: null,
      })
      .onConflictDoUpdate({
        target: [runtimeAgents.workspaceId, runtimeAgents.id],
        set: {
          profileYaml: agentYaml(parsedProfile),
          parsedProfile,
          updatedAt: new Date(),
          deletedAt: null,
        },
      });
  }
}

async function seedPolicies() {
  const policies = [
    {
      id: 'refund-guarantee',
      ownerAgentId: 'agt_demo_support',
      description: 'Block promises that guarantee refunds without approved policy context.',
      severity: 'high',
      action: 'block',
      enabled: true,
      literal: 'guaranteed refund',
    },
    {
      id: 'pii-leak',
      ownerAgentId: 'agt_demo_support',
      description: 'Escalate replies that include sensitive personal or payment data.',
      severity: 'critical',
      action: 'escalate',
      enabled: true,
      literal: 'credit card',
    },
    {
      id: 'medical-advice',
      ownerAgentId: 'agt_demo_billing',
      description: 'Escalate medical or health advice attempts to a human reviewer.',
      severity: 'critical',
      action: 'escalate',
      enabled: true,
      literal: 'medical advice',
    },
    {
      id: 'unapproved-discount',
      ownerAgentId: 'agt_demo_sales',
      description: 'Block discount commitments that are not in approved pricing docs.',
      severity: 'medium',
      action: 'block',
      enabled: false,
      literal: 'special discount',
    },
    {
      id: 'tone-softener',
      ownerAgentId: 'agt_demo_support',
      description: 'Rewrite dismissive support language into a calmer response.',
      severity: 'low',
      action: 'rewrite',
      enabled: true,
      literal: 'not our problem',
      rewrite: 'I understand this is frustrating. Let me route this to the right teammate.',
    },
  ] as const;

  for (const policy of policies) {
    const parsedPolicy = runtimePolicy(policy);
    await db
      .insert(runtimePolicies)
      .values({
        workspaceId: WORKSPACE_ID,
        id: policy.id,
        ownerAgentId: policy.ownerAgentId,
        enabled: policy.enabled,
        policyYaml: yamlPolicy(policy),
        parsedPolicy,
        updatedAt: new Date(),
        deletedAt: null,
      })
      .onConflictDoUpdate({
        target: [runtimePolicies.workspaceId, runtimePolicies.id],
        set: {
          ownerAgentId: policy.ownerAgentId,
          enabled: policy.enabled,
          policyYaml: yamlPolicy(policy),
          parsedPolicy,
          updatedAt: new Date(),
          deletedAt: null,
        },
      });
  }
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
  await db.delete(runtimeTraces).where(eq(runtimeTraces.workspaceId, WORKSPACE_ID));

  await db
    .insert(runtimeTraces)
    .values([
      {
        workspaceId: WORKSPACE_ID,
        traceId: '018f0f4a-4c0d-7000-9000-000000000001',
        domain: 'customer_support',
        decision: 'block',
        elapsedMs: 132,
        payload: tracePayload('018f0f4a-4c0d-7000-9000-000000000001', 'agt_demo_support', 'block', 132, 'refund-guarantee'),
        createdAt: minutesAgo(2),
      },
      {
        workspaceId: WORKSPACE_ID,
        traceId: '018f0f4a-4c0d-7000-9000-000000000002',
        domain: 'customer_support',
        decision: 'escalate',
        elapsedMs: 218,
        payload: tracePayload('018f0f4a-4c0d-7000-9000-000000000002', 'agt_demo_billing', 'escalate', 218, 'medical-advice'),
        createdAt: minutesAgo(8),
      },
      {
        workspaceId: WORKSPACE_ID,
        traceId: '018f0f4a-4c0d-7000-9000-000000000003',
        domain: 'customer_support',
        decision: 'escalate',
        elapsedMs: 177,
        payload: tracePayload('018f0f4a-4c0d-7000-9000-000000000003', 'agt_demo_support', 'escalate', 177, 'pii-leak'),
        createdAt: minutesAgo(14),
      },
      {
        workspaceId: WORKSPACE_ID,
        traceId: '018f0f4a-4c0d-7000-9000-000000000004',
        domain: 'customer_support',
        decision: 'rewrite',
        elapsedMs: 166,
        payload: tracePayload('018f0f4a-4c0d-7000-9000-000000000004', 'agt_demo_support', 'rewrite', 166, 'tone-softener'),
        createdAt: minutesAgo(21),
      },
      {
        workspaceId: WORKSPACE_ID,
        traceId: '018f0f4a-4c0d-7000-9000-000000000005',
        domain: 'customer_support',
        decision: 'allow',
        elapsedMs: 84,
        payload: tracePayload('018f0f4a-4c0d-7000-9000-000000000005', 'agt_demo_sales', 'allow', 84),
        createdAt: minutesAgo(35),
      },
    ])
    .onConflictDoNothing();
}

function runtimeAgent(
  id: string,
  displayName: string,
  scope: string,
  systemPrompt: string,
): RuntimeAgentProfile {
  return {
    agent_id: id,
    display_name: displayName,
    system_prompt: systemPrompt,
    scope: {
      in_scope: [scope],
      out_of_scope: ['medical advice', 'legal advice', 'guaranteed refunds'],
    },
    authority: {
      can_promise: ['approved help-center information', 'handoff to a teammate'],
      cannot_promise: ['refunds', 'medical outcomes', 'legal outcomes'],
    },
    tone: {
      target: 'clear-professional',
      forbidden: ['overconfident', 'dismissive'],
    },
    knowledge_sources: [],
    escalation_triggers: ['medical advice', 'legal advice', 'refund guarantee'],
  };
}

function agentYaml(agent: RuntimeAgentProfile): string {
  return `agent_id: ${agent.agent_id}
display_name: ${agent.display_name}
system_prompt: ${JSON.stringify(agent.system_prompt)}
scope:
  in_scope:
    - ${agent.scope?.in_scope?.[0] ?? 'customer support'}
  out_of_scope:
    - medical advice
    - legal advice
    - guaranteed refunds
authority:
  can_promise:
    - approved help-center information
    - handoff to a teammate
  cannot_promise:
    - refunds
    - medical outcomes
    - legal outcomes
tone:
  target: clear-professional
  forbidden:
    - overconfident
    - dismissive
knowledge_sources: []
escalation_triggers:
  - medical advice
  - legal advice
  - refund guarantee
`;
}

function runtimePolicy(policy: {
  id: string;
  ownerAgentId: string;
  description: string;
  severity: string;
  action: string;
  literal: string;
  rewrite?: string;
}): RuntimePolicyDocument {
  return {
    id: policy.id,
    description: policy.description,
    match: { literal: policy.literal },
    action: policy.action,
    severity: policy.severity,
    owner_agent_id: policy.ownerAgentId,
    ...(policy.rewrite ? { rewrite: policy.rewrite } : {}),
  };
}

function yamlPolicy(policy: {
  id: string;
  description: string;
  literal: string;
  action: string;
  severity: string;
  rewrite?: string;
}): string {
  return `id: ${policy.id}
description: ${policy.description}
match:
  literal: "${policy.literal}"
action: ${policy.action}
severity: ${policy.severity}
${policy.rewrite ? `rewrite: "${policy.rewrite}"\n` : ''}`;
}

function tracePayload(
  traceId: string,
  agentId: string,
  verdict: string,
  latencyMs: number,
  policyId?: string,
): RuntimeDecisionPayload {
  return {
    trace_id: traceId,
    verdict,
    reason: policyId ? `${policyId} triggered` : 'no policies triggered',
    triggered_policies: policyId
      ? [{ id: policyId, severity: 'medium', reason: `${policyId} matched` }]
      : [],
    safe_output: null,
    latency_ms: latencyMs,
    agent_id: agentId,
  };
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
