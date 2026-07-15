export interface UseCaseStep {
  label: string;
  title: string;
  body: string;
}

export interface UseCaseData {
  slug: 'ai-inference-spend' | 'x402-payments' | 'action-authorization' | 'email';
  href:
    | '/use-cases/ai-inference-spend'
    | '/use-cases/x402-payments'
    | '/use-cases/action-authorization'
    | '/use-case/email';
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
}

export const EMAIL_USE_CASE = {
  slug: 'email',
  href: '/use-case/email',
  number: '04',
  eyebrow: 'Email action control',
  title: 'Stop the wrong email before it leaves.',
  summary:
    'Let AI prepare the draft, then check the exact recipients, attachments, workflow state, approval, and prior attempts before an email provider makes the send real.',
  trigger:
    'An agent proposes an external email through Gmail, Outlook, a support platform, CRM, MCP tool, or email API.',
  failure:
    'The agent selects the wrong recipient, uses stale facts, includes the wrong attachment, changes an approved draft, or retries a send that already succeeded.',
  control:
    'Bind policy and approval to the exact proposed version, then allow, rewrite to draft, hold, or block before the provider send.',
  flow: ['Proposed email', 'Policy + context', 'Decision', 'Email provider'],
  steps: [
    {
      label: 'Describe',
      title: 'Turn the send into a typed action',
      body: 'Submit the actor, workflow, recipients, message and attachment hashes, triggering record, and an idempotency key before calling the email provider.',
    },
    {
      label: 'Check',
      title: 'Evaluate the exact external effect',
      body: 'Policy verifies recipient identity, workflow state, approval scope, changed fields, sensitive attachments, send velocity, and prior attempts outside the model prompt.',
    },
    {
      label: 'Hold',
      title: 'Keep uncertain sends as drafts',
      body: 'Low-risk sends can proceed. Risky or changed sends wait for a named reviewer, while denied sends can be rewritten into drafts instead of disappearing.',
    },
    {
      label: 'Prove',
      title: 'Join authorization to provider outcome',
      body: 'Record the approved version and provider message ID so a retry cannot silently become a duplicate and the team can see what actually happened.',
    },
  ],
  checks: [
    'Recipient and workflow match',
    'Approved version and attachments',
    'Duplicate intent and velocity',
    'External-send authority',
  ],
  result: 'Allow, draft, hold, or block before external send.',
  resultDetail:
    'Your existing email system still sends the message. TrustLoopGuard owns the contextual permission check, version-bound approval, duplicate protection, and decision receipt.',
  proof: ['Proposed send', 'Policy decision', 'Approved version', 'Provider outcome'],
  ctaLabel: 'Read the authorization model',
  ctaHref:
    'https://github.com/ducnguyen67201/TrustLoopGuard/blob/main/docs/concept/authorization.md',
} as const satisfies UseCaseData;

export const USE_CASES = [
  {
    slug: 'ai-inference-spend',
    href: '/use-cases/ai-inference-spend',
    number: '01',
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
    number: '02',
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
    number: '03',
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
  EMAIL_USE_CASE,
] as const satisfies readonly UseCaseData[];

export function getUseCase(slug: string): UseCaseData | undefined {
  return USE_CASES.find((useCase) => useCase.slug === slug);
}

export const USE_CASE_MENU_CLOSE_DELAY_MS = 220;

export const USE_CASE_NAV_ITEMS = [
  { href: '/use-cases', label: 'All use cases', detail: 'Choose a control boundary' },
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
  {
    href: '/use-case/email',
    label: 'Email action control',
    detail: 'Authorize before external send',
  },
] as const;

export const USE_CASE_NAV_GROUPS = {
  overview: USE_CASE_NAV_ITEMS[0],
  details: USE_CASE_NAV_ITEMS.slice(1),
} as const;
