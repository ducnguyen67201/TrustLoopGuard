export const activeOrganization = {
  id: 'org_acme',
  name: 'Acme Support',
  slug: 'acme-support',
};

export const workspaces = [
  {
    id: 'ws_support',
    name: 'Support AI',
    slug: 'support-ai',
    description: 'Customer support guardrails for billing, refunds, and account help.',
    policyCount: 12,
    enabledPolicies: 9,
    agentCount: 3,
    sourceCount: 5,
    apiKeyCount: 2,
    role: 'Owner',
  },
  {
    id: 'ws_sales',
    name: 'Sales Assistant',
    slug: 'sales-assistant',
    description: 'Pre-sales authority, pricing, and handoff boundaries.',
    policyCount: 7,
    enabledPolicies: 5,
    agentCount: 2,
    sourceCount: 3,
    apiKeyCount: 1,
    role: 'Admin',
  },
  {
    id: 'ws_internal',
    name: 'Internal Copilot',
    slug: 'internal-copilot',
    description: 'Employee productivity assistant for internal knowledge workflows.',
    policyCount: 4,
    enabledPolicies: 3,
    agentCount: 1,
    sourceCount: 2,
    apiKeyCount: 1,
    role: 'Viewer',
  },
];

export const activeWorkspace = workspaces[0]!;

export const guardrailMetrics = [
  {
    label: 'Decisions',
    value: '42,891',
    delta: '+18.4%',
    detail: 'Last 30 days',
  },
  {
    label: 'Blocked',
    value: '1,284',
    delta: '3.0%',
    detail: 'PII, refund, and legal claims',
  },
  {
    label: 'Escalated',
    value: '316',
    delta: '+6.2%',
    detail: 'Sent to human review',
  },
  {
    label: 'p95 latency',
    value: '184ms',
    delta: '-12ms',
    detail: 'Runtime guardrail checks',
  },
];

export const recentDecisions = [
  {
    id: 'trc_7K9',
    agent: 'Support bot',
    verdict: 'block',
    policy: 'refund-guarantee',
    latency: '132ms',
    time: '2m ago',
  },
  {
    id: 'trc_7K8',
    agent: 'Billing triage',
    verdict: 'escalate',
    policy: 'medical-advice',
    latency: '218ms',
    time: '8m ago',
  },
  {
    id: 'trc_7K7',
    agent: 'Support bot',
    verdict: 'allow',
    policy: 'baseline',
    latency: '77ms',
    time: '14m ago',
  },
  {
    id: 'trc_7K6',
    agent: 'Order status',
    verdict: 'rewrite',
    policy: 'tone-softener',
    latency: '166ms',
    time: '21m ago',
  },
];

export const agents = [
  {
    id: 'agt_support',
    name: 'Support bot',
    scope: 'Billing, account, and product support',
    policies: 6,
    status: 'Ready',
  },
  {
    id: 'agt_billing',
    name: 'Billing triage',
    scope: 'Invoices, plan changes, and refund routing',
    policies: 4,
    status: 'Needs review',
  },
  {
    id: 'agt_orders',
    name: 'Order status',
    scope: 'Shipping, delivery, and order lookup',
    policies: 2,
    status: 'Ready',
  },
];

export const knowledgeSources = [
  {
    id: 'src_refunds',
    title: 'Refund policy',
    kind: 'URL',
    location: 'help.acme.test/refunds',
    status: 'Ready',
    lastIndexed: 'Today 09:42',
  },
  {
    id: 'src_pricing',
    title: 'Pricing exceptions',
    kind: 'Note',
    location: 'Manual workspace note',
    status: 'Ready',
    lastIndexed: 'Yesterday',
  },
  {
    id: 'src_security',
    title: 'Security FAQ',
    kind: 'File',
    location: 'security-faq.pdf',
    status: 'Indexing',
    lastIndexed: 'In progress',
  },
];

export const apiKeys = [
  {
    id: 'key_live',
    name: 'Production runtime',
    prefix: 'tlg_live_8a4f',
    status: 'Active',
    lastUsed: '4m ago',
    createdBy: 'Mina Chen',
  },
  {
    id: 'key_ci',
    name: 'CI policy tests',
    prefix: 'tlg_live_12be',
    status: 'Active',
    lastUsed: '2d ago',
    createdBy: 'Ravi Patel',
  },
];

export const teamMembers = [
  {
    id: 'usr_mina',
    name: 'Mina Chen',
    email: 'mina@acme.test',
    role: 'Owner',
    access: 'All workspaces',
  },
  {
    id: 'usr_ravi',
    name: 'Ravi Patel',
    email: 'ravi@acme.test',
    role: 'Admin',
    access: 'Support AI, Sales Assistant',
  },
  {
    id: 'usr_lee',
    name: 'Lee Carter',
    email: 'lee@acme.test',
    role: 'Viewer',
    access: 'Internal Copilot',
  },
];
