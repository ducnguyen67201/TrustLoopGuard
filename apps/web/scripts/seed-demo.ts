const SERVER_URL = process.env['NEXT_PUBLIC_TL_SERVER_URL'] ?? 'http://127.0.0.1:3001';
const WORKSPACE_ID = process.env['TL_DEMO_WORKSPACE_ID'] ?? 'ws_trustloop_demo';
const API_KEY = process.env['TL_API_KEY'];

interface DemoAgentProfile {
  agent_id: string;
  display_name: string;
  scope: {
    in_scope: string[];
    out_of_scope: string[];
  };
  authority: {
    can_promise: string[];
    cannot_promise: string[];
  };
  tone: {
    target: string;
    forbidden: string[];
  };
  knowledge_sources: string[];
  escalation_triggers: string[];
}

interface DemoKnowledgeSource {
  title: string;
  kind: string;
  location: string;
  notes: string;
}

interface DemoTraceInput {
  agent_id: string;
  channel: string;
  domain: string;
  input: string;
  proposed_output: string;
}

interface DemoToolMetadata {
  tool: string;
  side_effect:
    | 'none'
    | 'read'
    | 'external_communication'
    | 'file_write'
    | 'shell_exec'
    | 'network_call'
    | 'db_mutation'
    | 'api_mutation'
    | 'memory_write'
    | 'publish';
  reversible: boolean;
  params: Array<{
    path: string;
    role: 'authority_bearing' | 'content_bearing';
    allowed_sources: Array<{
      origin: 'user' | 'system' | 'tool' | 'memory' | 'file' | 'web' | 'email' | 'api' | 'unknown';
      kind?: string;
    }>;
  }>;
}

async function main() {
  await upsertAgent({
    agent_id: 'support-bot',
    display_name: 'Support bot',
    scope: {
      in_scope: ['billing questions', 'refund policy lookup', 'product support'],
      out_of_scope: ['medical advice', 'legal advice'],
    },
    authority: {
      can_promise: ['approved refunds', 'documented discounts'],
      cannot_promise: ['guaranteed refunds', 'unapproved credits'],
    },
    tone: {
      target: 'calm and concise',
      forbidden: ['dismissive', 'combative'],
    },
    knowledge_sources: [],
    escalation_triggers: ['sensitive data', 'medical advice'],
  });

  await upsertPolicy(`
id: pii-leak
description: Escalate replies that include sensitive personal or payment data.
match:
  regex: "(ssn|social security|credit card|card number)"
action: escalate
severity: critical
owner_agent_id: support-bot
`);

  await upsertPolicy(`
id: refund-guarantee
description: Block promises that guarantee refunds without approved policy context.
match:
  literal: guaranteed refund
action: block
severity: high
owner_agent_id: support-bot
`);

  await upsertToolMetadata({
    tool: 'send_email',
    side_effect: 'external_communication',
    reversible: false,
    params: [
      {
        path: 'to',
        role: 'authority_bearing',
        allowed_sources: [{ origin: 'user' }, { origin: 'tool', kind: 'contact_lookup' }],
      },
      {
        path: 'body',
        role: 'content_bearing',
        allowed_sources: [
          { origin: 'user' },
          { origin: 'tool', kind: 'approved_document_summary' },
        ],
      },
    ],
  });

  await upsertToolMetadata({
    tool: 'update_tax_record',
    side_effect: 'db_mutation',
    reversible: false,
    params: [
      {
        path: 'status',
        role: 'authority_bearing',
        allowed_sources: [{ origin: 'user' }, { origin: 'tool', kind: 'tax_review_system' }],
      },
    ],
  });

  await upsertToolMetadata({
    tool: 'post_webhook',
    side_effect: 'network_call',
    reversible: false,
    params: [
      {
        path: 'url',
        role: 'authority_bearing',
        allowed_sources: [{ origin: 'user' }, { origin: 'tool', kind: 'approved_webhook' }],
      },
      {
        path: 'body',
        role: 'content_bearing',
        allowed_sources: [
          { origin: 'user' },
          { origin: 'tool', kind: 'approved_document_summary' },
        ],
      },
    ],
  });

  await upsertToolMetadata({
    tool: 'create_review_task',
    side_effect: 'memory_write',
    reversible: true,
    params: [
      {
        path: 'title',
        role: 'content_bearing',
        allowed_sources: [{ origin: 'file', kind: 'document' }, { origin: 'user' }],
      },
      {
        path: 'assignee',
        role: 'authority_bearing',
        allowed_sources: [{ origin: 'system' }],
      },
    ],
  });

  await enforceDemoGuardSettings();

  await createKnowledgeSource({
    title: 'Refund policy',
    kind: 'note',
    location: 'Internal policy note',
    notes: 'Refunds require approved policy context before commitments are made.',
  });

  await recordTrace({
    agent_id: 'support-bot',
    channel: 'chat',
    domain: 'customer_support',
    input: 'Can you guarantee a refund?',
    proposed_output: 'I can guarantee a full refund today.',
  });
}

async function upsertAgent(profile: DemoAgentProfile) {
  await request('/v1/agents', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(profile),
  });
}

async function upsertPolicy(yaml: string) {
  await request('/v1/policies', {
    method: 'POST',
    headers: { 'content-type': 'application/yaml' },
    body: yaml.trim(),
  });
}

async function createKnowledgeSource(source: DemoKnowledgeSource) {
  await request('/v1/knowledge-sources', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(source),
  });
}

async function upsertToolMetadata(metadata: DemoToolMetadata) {
  await request('/v1/tool-metadata', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(metadata),
  });
}

async function enforceDemoGuardSettings() {
  await request('/v1/settings', {
    method: 'PATCH',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({
      param_checker_mode: 'enforce',
    }),
  });
}

async function recordTrace(trace: DemoTraceInput) {
  await request('/v1/events', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({
      kind: 'output.proposed',
      principal: {
        workspace_id: '',
        environment_id: '',
        agent_id: trace.agent_id,
      },
      action: {
        operation: 'output',
        parameters: { text: trace.proposed_output },
        side_effect: 'none',
      },
      sources: [
        {
          id: 'input.observed',
          origin: 'user',
          labels: {},
          kind: 'demo.input',
        },
        {
          id: 'model.output',
          origin: 'unknown',
          labels: {},
          kind: 'demo.output',
        },
      ],
      provenance: {
        text: ['model.output'],
      },
      context: {
        channel: trace.channel,
        domain: trace.domain,
        input_text: trace.input,
      },
    }),
  });
}

async function request(path: string, init: RequestInit) {
  const headers = new Headers(init.headers);
  headers.set('x-tlg-workspace-id', WORKSPACE_ID);
  if (API_KEY) headers.set('authorization', `Bearer ${API_KEY}`);

  const res = await fetch(`${SERVER_URL}${path}`, {
    ...init,
    headers,
  });
  if (!res.ok) {
    const body = await res.text().catch(() => '');
    throw new Error(`${path} failed with ${res.status}: ${body}`);
  }
}

main()
  .then(() => {
    console.log(`Seeded demo data through tl-server for workspace ${WORKSPACE_ID}.`);
  })
  .catch((error) => {
    console.error(error);
    process.exitCode = 1;
  });
