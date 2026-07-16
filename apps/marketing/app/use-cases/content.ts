export interface UseCaseStep {
  label: string;
  title: string;
  body: string;
}

export type UseCaseDemoEffect = 'permit' | 'transform' | 'require_approval' | 'deny';

export interface UseCaseDemoField {
  label: string;
  value: string;
}

export interface UseCaseDemoDecision {
  subject: string;
  effect: UseCaseDemoEffect;
  detail: string;
}

export interface UseCaseDemo {
  kind: 'shell' | 'email' | 'spend';
  proposalTitle: string;
  proposalCode: string;
  proposalFields: readonly UseCaseDemoField[];
  policyTitle: string;
  policyFields: readonly UseCaseDemoField[];
  decisions: readonly UseCaseDemoDecision[];
  executionTitle: string;
  executionDetail: string;
  boundary: string;
}

export type UseCaseSlug =
  | 'shell-command-safety'
  | 'email'
  | 'agent-spending-caps'
  | 'ai-inference-spend'
  | 'x402-payments'
  | 'action-authorization';

export interface UseCaseData {
  slug: UseCaseSlug;
  href: `/use-cases/${UseCaseSlug}`;
  number: string;
  eyebrow: string;
  title: string;
  summary: string;
  trigger: string;
  failure: string;
  control: string;
  flow: readonly string[];
  steps: readonly UseCaseStep[];
  checks: readonly string[];
  result: string;
  resultDetail: string;
  proof: readonly string[];
  ctaLabel: string;
  ctaHref: string;
  demo?: UseCaseDemo;
}

export const SHELL_COMMAND_USE_CASE = {
  slug: 'shell-command-safety',
  href: '/use-cases/shell-command-safety',
  number: '01',
  eyebrow: 'Shell command guardrails',
  title: 'Stop dangerous shell commands before they run.',
  summary:
    'Evaluate each proposed Bash, sh, or zsh action as structured input, then deny it or require exact-action approval before the coding agent can execute it.',
  trigger:
    'A coding agent proposes a shell action through Claude Code, an SDK integration, or another tool adapter.',
  failure:
    'A destructive command reaches root, system paths, repository history, production infrastructure, or another sensitive target before a person sees it.',
  control:
    'Match deterministic shell facts and exact parameters against enabled tool policies, then deny, defer, or require approval before the executor runs the command.',
  flow: ['Proposed command', 'Policy + shell facts', 'Decision', 'Agent executor'],
  steps: [
    {
      label: 'Describe',
      title: 'Capture the exact proposed action',
      body: 'Submit the command, shell, working directory, workspace root, timeout, stable invocation, and complete tool identity before execution.',
    },
    {
      label: 'Analyze',
      title: 'Derive bounded shell facts',
      body: 'TrustLoopGuard parses executable syntax without running the command, then identifies targets, wrappers, destructive operations, dynamic evaluation, and incomplete analysis.',
    },
    {
      label: 'Decide',
      title: 'Apply the enabled tool policy',
      body: 'Known prohibited actions are denied. Workspace-sensitive actions can wait for an exact, non-reusable approval that is bound to the original parameters.',
    },
    {
      label: 'Prove',
      title: 'Tie approval to one execution attempt',
      body: 'The caller resubmits the same action with its grant. A changed command does not fit, and the execution lease is consumed or canceled after the attempt.',
    },
  ],
  checks: [
    'Tool identity and exact command',
    'Deterministic shell risk facts',
    'Workspace and target scope',
    'Approval and execution lease',
  ],
  result: 'Deny, hold, or permit before execution.',
  resultDetail:
    'The coding agent still owns execution. TrustLoopGuard treats the command as structured data, returns the policy decision, and never runs the shell while analyzing it.',
  proof: ['Proposed command', 'Shell facts', 'Policy finding', 'Lease outcome'],
  ctaLabel: 'See shell command safety',
  ctaHref:
    'https://github.com/ducnguyen67201/TrustLoopGuard/blob/main/docs/concept/command-safety.md#operator-demo',
  demo: {
    kind: 'shell',
    proposalTitle: 'Bash proposes a destructive action',
    proposalCode: 'rm -rf /',
    proposalFields: [
      { label: 'Tool', value: 'claude-code / Bash' },
      { label: 'Workspace', value: '/workspace/project' },
    ],
    policyTitle: 'Tool policy matches shell facts',
    policyFields: [
      { label: 'Risk', value: 'filesystem_recursive_delete' },
      { label: 'Target', value: 'root' },
      { label: 'Action', value: 'deny' },
    ],
    decisions: [
      { subject: 'rm -rf /', effect: 'deny', detail: 'System target blocked' },
      {
        subject: 'rm -rf ./build',
        effect: 'require_approval',
        detail: 'Exact action waits',
      },
    ],
    executionTitle: 'The executor stays paused',
    executionDetail:
      'Denied commands never run. An approved command receives one action-bound execution lease.',
    boundary: 'Analysis parses the command as structured input and never invokes the shell.',
  },
} as const satisfies UseCaseData;

export const EMAIL_USE_CASE = {
  slug: 'email',
  href: '/use-cases/email',
  number: '02',
  eyebrow: 'Outbound email guardrails',
  title: 'Rewrite risky emails before they send.',
  summary:
    'Scope a content policy to email so safe drafts pass unchanged and risky promises are replaced with policy-approved language before delivery.',
  trigger:
    'An agent proposes a customer-facing email through the SDK, gateway, support workflow, CRM, or another application integration.',
  failure:
    'A draft guarantees a refund, exposes sensitive information, violates policy, or uses wording the business cannot stand behind.',
  control:
    'Evaluate the proposed message against the email-scoped content policy, then return permit or a safe rewrite before the customer mailer sends anything.',
  flow: ['Proposed email', 'Email policy', 'Permit or rewrite', 'Customer mailer'],
  steps: [
    {
      label: 'Choose',
      title: 'Scope a policy to outbound email',
      body: 'Set the email channel, the risky wording or semantic match, the desired action, and a policy-approved replacement when the action is transform.',
    },
    {
      label: 'Publish',
      title: 'Manage one policy through the registry',
      body: 'Validate and publish the YAML, then confirm it is enabled for the intended environment and agent before customer-facing traffic reaches it.',
    },
    {
      label: 'Propose',
      title: 'Check the message before delivery',
      body: 'Submit the proposed output with context.channel set to email. A safe draft returns permit; risky wording returns transform with the configured rewrite.',
    },
    {
      label: 'Apply',
      title: 'Send only the policy-safe result',
      body: 'The customer application keeps ownership of delivery and applies the returned decision before calling Gmail, Outlook, a support platform, or another provider.',
    },
  ],
  checks: [
    'Email channel and agent scope',
    'Risky wording or semantic match',
    'Configured policy action',
    'Policy-approved replacement',
  ],
  result: 'Permit the safe draft or return a policy-approved rewrite.',
  resultDetail:
    'Your existing email system still sends the message. TrustLoopGuard evaluates the proposed content and never sends the email itself.',
  proof: ['Original draft', 'Matched policy', 'Decision reason', 'Safe replacement'],
  ctaLabel: 'Try the email policy demo',
  ctaHref:
    'https://github.com/ducnguyen67201/TrustLoopGuard/blob/main/docs/policies/README.md#email-policy-demo',
  demo: {
    kind: 'email',
    proposalTitle: 'The agent proposes a customer email',
    proposalCode: 'This is a guaranteed refund.',
    proposalFields: [
      { label: 'Operation', value: 'send_email' },
      { label: 'Channel', value: 'email' },
    ],
    policyTitle: 'The email policy checks the draft',
    policyFields: [
      { label: 'Match', value: 'guaranteed refund' },
      { label: 'Action', value: 'transform' },
      { label: 'Scope', value: 'support-agent' },
    ],
    decisions: [
      { subject: 'Safe draft', effect: 'permit', detail: 'Send unchanged' },
      { subject: 'Risky promise', effect: 'transform', detail: 'Use safe replacement' },
    ],
    executionTitle: 'The customer mailer applies the result',
    executionDetail:
      'The application sends the original permitted draft or the policy-approved replacement.',
    boundary: 'TrustLoopGuard evaluates the proposed message and never sends the email.',
  },
} as const satisfies UseCaseData;

export const AGENT_SPENDING_CAPS_USE_CASE = {
  slug: 'agent-spending-caps',
  href: '/use-cases/agent-spending-caps',
  number: '03',
  eyebrow: 'Agent spending caps',
  title: 'Enforce agent spending caps before payment.',
  summary:
    'Use one financial policy to permit routine spend, hold an exception for approval, and deny a payment that breaches the hard cap.',
  trigger:
    'An agent proposes a typed payment, purchase, refund, payout, or vendor action before provider execution.',
  failure:
    'Routine spend, reviewable exceptions, and true cap breaches collapse into the same path, leaving the agent or prompt to police its own authority.',
  control:
    'Evaluate the exact principal, operation, amount, currency, rail, approval threshold, and live budget before execution is allowed to start.',
  flow: ['Proposed payment', 'Financial policy', 'Decision', 'Payment provider'],
  steps: [
    {
      label: 'Configure',
      title: 'Set the hard and human boundaries',
      body: 'Choose the agent, operation, rail, currency, per-action cap, rolling budget, and the amount above which a named reviewer must approve.',
    },
    {
      label: 'Evaluate',
      title: 'Submit the payment without executing it',
      body: 'Create typed financial actions with execute set to false so policy, evidence, eligibility, and current budget are checked before any provider call.',
    },
    {
      label: 'Hold',
      title: 'Separate exceptions from breaches',
      body: 'Routine spend can be authorized, an in-policy exception can wait in the approvals queue, and a hard-cap breach is blocked immediately.',
    },
    {
      label: 'Execute',
      title: 'Recheck before money moves',
      body: 'Approved actions are re-evaluated against current policy and live budget before the financial service reserves funds and calls the configured provider.',
    },
  ],
  checks: [
    'Principal and operation',
    'Currency, rail, and counterparty',
    'Per-action and rolling caps',
    'Approval threshold and live budget',
  ],
  result: '$25 permit. $75 hold. $150 deny.',
  resultDetail:
    'The payment provider still moves the money. TrustLoopGuard owns the pre-spend decision, approval requirement, live budget check, and linked authorization receipt.',
  proof: ['Financial action', 'Policy finding', 'Reviewer grant', 'Execution receipt'],
  ctaLabel: 'See the spending cap demo',
  ctaHref:
    'https://github.com/ducnguyen67201/TrustLoopGuard/blob/main/docs/concept/financial-authorization.md#spending-cap-demo',
  demo: {
    kind: 'spend',
    proposalTitle: 'The agent proposes a vendor payment',
    proposalCode: '$75.00 USD',
    proposalFields: [
      { label: 'Operation', value: 'pay_vendor' },
      { label: 'Principal', value: 'spend-agent' },
    ],
    policyTitle: 'Financial policy checks authority',
    policyFields: [
      { label: 'Per action', value: '$100 hard cap' },
      { label: 'Review above', value: '$50' },
      { label: 'Monthly', value: '$1,000' },
    ],
    decisions: [
      { subject: '$25 routine', effect: 'permit', detail: 'Authorized' },
      { subject: '$75 exception', effect: 'require_approval', detail: 'Held for review' },
      { subject: '$150 over cap', effect: 'deny', detail: 'Blocked' },
    ],
    executionTitle: 'The provider call waits',
    executionDetail:
      'Only a currently authorized action can reserve live budget and reach the payment provider.',
    boundary: 'Authorization analysis never executes the payment.',
  },
} as const satisfies UseCaseData;

export const USE_CASES = [
  SHELL_COMMAND_USE_CASE,
  EMAIL_USE_CASE,
  AGENT_SPENDING_CAPS_USE_CASE,
  {
    slug: 'ai-inference-spend',
    href: '/use-cases/ai-inference-spend',
    number: '04',
    eyebrow: 'AI inference spend',
    title: 'Put a hard ceiling on model usage.',
    summary:
      'Route OpenAI-compatible model traffic through TrustLoopGuard to meter cost, warn before the limit, and stop new requests before provider spend crosses the cap.',
    trigger: 'Every model request sent through a TrustLoopGuard gateway route.',
    failure:
      'An agent loop, user, or team keeps consuming tokens while the provider dashboard only explains the bill after the fact.',
    control:
      'Customer-owned model pricing, a visible usage dashboard, an 80% alert, and a hard cap enforced before the upstream call.',
    flow: ['Your app', 'TrustLoopGuard gateway', 'Budget decision', 'Model provider'],
    steps: [
      {
        label: 'Route',
        title: 'Keep the client shape you already use',
        body: 'Point the OpenAI-compatible base URL at TrustLoopGuard. The route binds the caller, agent, model, and provider without changing the chat payload.',
      },
      {
        label: 'Meter',
        title: 'Turn tokens into attributable spend',
        body: 'Price and usage are recorded by model, principal, request, and budget window so the operator can see who consumed what.',
      },
      {
        label: 'Warn',
        title: 'Fire the 80% threshold once',
        body: 'A budget alert warns the team while there is still room to investigate. The alert informs; it does not replace enforcement.',
      },
      {
        label: 'Stop',
        title: 'Reject the request before provider spend',
        body: 'When the hard cap would be exceeded, TrustLoopGuard returns budget_exceeded and never forwards the request upstream.',
      },
    ],
    checks: ['Caller and agent', 'Exact model price', 'Committed spend', 'Requested token ceiling'],
    result: 'Dashboard + 80% alert + hard cutoff.',
    resultDetail:
      'The operator can see usage as it happens, get an early warning, and know the cap is owned outside the same provider sending the bill.',
    proof: [
      'Per-request usage',
      'Caller and model attribution',
      'Budget decision',
      'Provider response ID',
    ],
    ctaLabel: 'Read the gateway setup',
    ctaHref: 'https://github.com/ducnguyen67201/TrustLoopGuard#gateway-proxy-quickstart',
  },
  {
    slug: 'x402-payments',
    href: '/use-cases/x402-payments',
    number: '05',
    eyebrow: 'x402 agent payments',
    title: 'Authorize the purchase before the agent signs.',
    summary:
      'An x402 rail tells the agent how to pay. TrustLoopGuard decides whether this agent should pay this amount, to this endpoint, for this task, right now.',
    trigger: 'An agent receives an HTTP 402 payment requirement for a paid API or tool.',
    failure:
      'Parallel discovery, duplicate retries, merchant drift, or a runaway tool loop consumes the session budget without a reliable pre-spend decision.',
    control:
      'Normalize the payment requirement, verify the grant, reserve session budget, and return a signable decision only when every boundary passes.',
    flow: ['402 requirement', 'Authorize + reserve', 'Agent signs', 'Commit receipt'],
    steps: [
      {
        label: 'Propose',
        title: 'Submit the exact payment requirement',
        body: 'The runtime sends amount, payee, network, asset, resource, purpose, principal, and session budget before signing anything.',
      },
      {
        label: 'Authorize',
        title: 'Check grant and standing policy',
        body: 'TrustLoopGuard compares the request with the allowed host, resource, network, asset, counterparty, amount, and task scope.',
      },
      {
        label: 'Reserve',
        title: 'Hold budget across concurrent calls',
        body: 'An action-bound reservation prevents parallel requests or stale balance reads from spending the same remaining budget twice.',
      },
      {
        label: 'Settle',
        title: 'Commit only the authorized payment',
        body: 'The settlement proof must match the normalized requirement. Commit closes the reservation and creates the execution receipt.',
      },
    ],
    checks: ['Grant scope', 'Endpoint and payee', 'Session budget', 'Duplicate requirement hash'],
    result: 'Allow, hold, or block before wallet signing.',
    resultDetail:
      'The payment rail still moves the money. TrustLoopGuard owns the pre-spend judgment, concurrent budget reservation, and proof of why the payment was authorized.',
    proof: [
      'Normalized requirement',
      'Policy and grant result',
      'Reservation state',
      'Settlement receipt',
    ],
    ctaLabel: 'Inspect x402 authorization',
    ctaHref:
      'https://github.com/ducnguyen67201/TrustLoopGuard/blob/main/docs/concept/financial-authorization.md#agentic-x402-payments',
  },
  {
    slug: 'action-authorization',
    href: '/use-cases/action-authorization',
    number: '06',
    eyebrow: 'Action authorization',
    title: 'Guard the one-way door in any agent workflow.',
    summary:
      'Let AI prepare the work, then put a deterministic or human decision before the refund, invoice approval, payout, booking, account change, or other irreversible commit.',
    trigger:
      'An agent proposes a consequential action through an SDK, tool wrapper, gateway, or API adapter.',
    failure:
      'The final answer looks reasonable, but an intermediate tool call crosses a policy boundary, lacks trusted evidence, or exceeds the authority the principal granted.',
    control:
      'Create a typed proposed action, evaluate authority and evidence, then authorize, hold for approval, or deny before the existing system executes it.',
    flow: ['Proposed action', 'Policy + evidence', 'Decision', 'Existing system'],
    steps: [
      {
        label: 'Describe',
        title: 'Make the action explicit',
        body: 'Send the operation, amount, principal, counterparty, grant, evidence references, and idempotency key as a typed action.',
      },
      {
        label: 'Decide',
        title: 'Evaluate facts outside the prompt',
        body: 'Deterministic policy checks the action against caps, spend windows, required evidence, known counterparties, and approval thresholds.',
      },
      {
        label: 'Escalate',
        title: 'Hold the ambiguous cases',
        body: 'A held action does not execute. It waits for the named approver, and the approval or denial becomes part of the record.',
      },
      {
        label: 'Prove',
        title: 'Join decision to execution',
        body: 'The decision receipt explains what was allowed. The execution receipt records what actually moved after authorization.',
      },
    ],
    checks: ['Principal authority', 'Action policy', 'Trusted evidence', 'Approval requirement'],
    result: 'Authorize, hold, or deny before the one-way door.',
    resultDetail:
      'Your existing system still performs the action. TrustLoopGuard supplies the control boundary and receipts that show who approved what, under which policy, and with which evidence.',
    proof: ['Proposed action', 'Decision reason', 'Approver identity', 'Execution outcome'],
    ctaLabel: 'Read the authorization contract',
    ctaHref:
      'https://github.com/ducnguyen67201/TrustLoopGuard/blob/main/docs/concept/financial-authorization.md',
  },
] as const satisfies readonly UseCaseData[];

export function getUseCase(slug: string): UseCaseData | undefined {
  return USE_CASES.find((useCase) => useCase.slug === slug);
}

export const USE_CASE_MENU_CLOSE_DELAY_MS = 220;

export const USE_CASE_NAV_ITEMS = [
  { href: '/use-cases', label: 'All use cases', detail: 'Choose a control boundary' },
  {
    href: '/use-cases/shell-command-safety',
    label: 'Shell command safety',
    detail: 'Deny or approve before execution',
  },
  {
    href: '/use-cases/email',
    label: 'Outbound email',
    detail: 'Permit or rewrite before send',
  },
  {
    href: '/use-cases/agent-spending-caps',
    label: 'Agent spending caps',
    detail: 'Permit, hold, or deny payment',
  },
  {
    href: '/use-cases/ai-inference-spend',
    label: 'AI inference spend',
    detail: 'Meter, alert, and hard cap',
  },
  {
    href: '/use-cases/x402-payments',
    label: 'x402 agent payments',
    detail: 'Authorize before wallet signing',
  },
  {
    href: '/use-cases/action-authorization',
    label: 'Action authorization',
    detail: 'Guard the one-way door',
  },
] as const;

export const USE_CASE_NAV_GROUPS = {
  overview: USE_CASE_NAV_ITEMS[0],
  details: USE_CASE_NAV_ITEMS.slice(1),
} as const;
