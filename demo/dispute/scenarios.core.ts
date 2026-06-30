// Pure, importable core of the money-agent showcase: the scenario list, the
// GuardEvent builder, and the verdict-gated runner. No I/O at module load, so
// tests can import it without triggering a run (scenarios.ts is the entry).

import type { Decision, GuardEvent, Source } from '@trustloopguard/sdk';

import type { PaymentRequest, PaymentResult } from './payments';

export interface Scenario {
  label: string;
  tool: 'issue_refund' | 'wire_transfer';
  amountCents: number;
  /** Destination from the account registry (trusted) vs injected from chat. */
  destinationTrusted: boolean;
  /** Send a non-integer amount to trip the "unverifiable" escalation. */
  ambiguousAmount?: boolean;
  expectedControl: Control;
}

export type Control = 'value_limit' | 'parameter_auth' | 'approval' | 'none';

// Each scenario is tuned so exactly one control decides it. Scenario 2 uses a
// TRUSTED destination so only value_limit can trip (an injected destination
// would also block via parameter_auth and muddy the lesson).
export const SCENARIOS: Scenario[] = [
  { label: 'legit refund $50', tool: 'issue_refund', amountCents: 5_000, destinationTrusted: true, expectedControl: 'none' },
  { label: 'over-cap refund $750', tool: 'issue_refund', amountCents: 75_000, destinationTrusted: true, expectedControl: 'value_limit' },
  { label: 'refund to injected account', tool: 'issue_refund', amountCents: 5_000, destinationTrusted: false, expectedControl: 'parameter_auth' },
  { label: 'ambiguous (non-integer) amount', tool: 'issue_refund', amountCents: 5_000, destinationTrusted: true, ambiguousAmount: true, expectedControl: 'value_limit' },
  { label: 'wire transfer needs sign-off', tool: 'wire_transfer', amountCents: 5_000, destinationTrusted: true, expectedControl: 'approval' },
];

const TRUSTED_SOURCE: Source = {
  id: 'account_registry',
  origin: 'tool',
  kind: 'account_registry',
  labels: { trust: 'trusted', confidentiality: 'unknown', integrity: 'high' },
};

const UNTRUSTED_SOURCE: Source = {
  id: 'conversation',
  origin: 'user',
  labels: { trust: 'untrusted', confidentiality: 'unknown', integrity: 'unknown' },
};

function destinationValue(scenario: Scenario): string {
  return scenario.destinationTrusted ? 'acct_registry_001' : 'attacker@evil.example';
}

export function buildEvent(scenario: Scenario, agentId: string): GuardEvent {
  const source = scenario.destinationTrusted ? TRUSTED_SOURCE : UNTRUSTED_SOURCE;
  const destination = destinationValue(scenario);
  // Cents are integers; the ambiguous case sends a non-integer the guard can't verify.
  const amount = scenario.ambiguousAmount === true ? scenario.amountCents + 0.5 : scenario.amountCents;
  const parameters =
    scenario.tool === 'issue_refund'
      ? { amount, account: destination, reason: 'demo scenario' }
      : { amount, destination, reason: 'demo scenario' };
  const destParam = scenario.tool === 'issue_refund' ? 'account' : 'destination';

  return {
    kind: 'tool.call.proposed',
    principal: { workspace_id: '', environment_id: '', agent_id: agentId },
    action: { operation: scenario.tool, parameters, side_effect: 'api_mutation' },
    sources: [source],
    provenance: { amount: [source.id], [destParam]: [source.id] },
    context: { channel: 'chat', domain: 'payments', product: 'Money Agent demo' },
  };
}

/** The control a decision was actually decided by, read from the verdict evidence. */
export function controlFor(decision: Decision): Control {
  const rule = decision.violated_rule ?? '';
  if (rule.startsWith('parameter_value')) return 'value_limit';
  if (rule.startsWith('parameter_source')) return 'parameter_auth';
  if (rule.startsWith('approval')) return 'approval';
  return 'none';
}

export interface ScenarioRow {
  label: string;
  verdict: Decision['verdict'];
  control: Control;
  result: string;
}

export type SubmitFn = (event: GuardEvent) => Promise<Decision>;
export type PayFn = (req: PaymentRequest) => Promise<PaymentResult>;

export async function runScenarios(deps: {
  agentId: string;
  submit: SubmitFn;
  pay: PayFn;
}): Promise<ScenarioRow[]> {
  const rows: ScenarioRow[] = [];
  for (const scenario of SCENARIOS) {
    const decision = await deps.submit(buildEvent(scenario, deps.agentId));
    let result: string;
    if (decision.verdict === 'allow') {
      const payment = await deps.pay({
        kind: scenario.tool === 'wire_transfer' ? 'wire' : 'refund',
        amountCents: scenario.amountCents,
        destination: destinationValue(scenario),
      });
      result = payment.mode === 'simulated' ? 'paid (simulated)' : `paid: ${payment.ref}`;
    } else {
      result = 'stopped before payment';
    }
    rows.push({ label: scenario.label, verdict: decision.verdict, control: controlFor(decision), result });
  }
  return rows;
}
