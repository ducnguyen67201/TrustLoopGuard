'use client';

import { useState, type FormEvent } from 'react';

import { trackMarketingEvent } from '@/lib/gtm';

import {
  PROCUREMENT_POLICY_IDS,
  sanitizeProcurementDemoResponse,
  type JsonValue,
  type ProcurementDemoResponse,
  type ProcurementPolicyId,
} from './contract';
import styles from './procurement.module.css';

const EXAMPLES = [
  {
    label: 'Approved chairs · allow',
    prompt: 'Order the approved office chairs for $2,400.',
  },
  {
    label: 'Laptops · review',
    prompt: 'Order the approved developer laptops for $42,000.',
  },
  {
    label: 'Unapproved vendor · block',
    prompt: 'Order the office supplies from the unapproved vendor.',
  },
  {
    label: 'Gift cards · block',
    prompt: 'Order the employee gift cards from the approved vendor.',
  },
] as const;

const POLICY_COPY: Readonly<
  Record<ProcurementPolicyId, { title: string; description: string; effect: 'Deny' | 'Review' }>
> = {
  'procurement-approved-suppliers': {
    title: 'Approved suppliers only',
    description: 'Block purchase orders from vendors outside the approved supplier list.',
    effect: 'Deny',
  },
  'procurement-high-value-review': {
    title: 'Review high-value orders',
    description: 'Require an owner or administrator to approve high-value purchase orders.',
    effect: 'Review',
  },
  'procurement-restricted-categories': {
    title: 'Block restricted categories',
    description: 'Stop gift cards and other categories procurement does not permit.',
    effect: 'Deny',
  },
};

type RunState = 'idle' | 'running' | 'success' | 'error';
type StepState = 'idle' | 'running' | 'complete' | 'stopped';

export function ProcurementDemo() {
  const [prompt, setPrompt] = useState<string>(EXAMPLES[1].prompt);
  const [submittedPrompt, setSubmittedPrompt] = useState('');
  const [runState, setRunState] = useState<RunState>('idle');
  const [response, setResponse] = useState<ProcurementDemoResponse | null>(null);
  const [error, setError] = useState('');
  const [selectedPolicies, setSelectedPolicies] = useState<Set<ProcurementPolicyId>>(
    () => new Set(PROCUREMENT_POLICY_IDS),
  );

  const activePolicyIds = PROCUREMENT_POLICY_IDS.filter((policyId) =>
    selectedPolicies.has(policyId),
  );
  const decision = response?.result.decision;
  const purchaseOrder = response?.state.purchaseOrders[0];

  async function runDemo(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const message = prompt.trim();
    if (message === '' || runState === 'running') return;
    const scenario = demoScenario(message);

    trackMarketingEvent('demo_started', {
      page: '/demo/procurement',
      location: 'procurement_composer',
      scenario,
      label: activePolicyIds.join(','),
    });

    setRunState('running');
    setSubmittedPrompt(message);
    setResponse(null);
    setError('');

    try {
      const result = await fetch('/api/demo/procurement', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ prompt: message, activePolicyIds }),
      });
      const payload: JsonValue = await result.json().catch(() => null);
      if (!result.ok) {
        throw new Error(publicError(payload) ?? 'The live procurement workflow failed safely.');
      }
      const body = sanitizeProcurementDemoResponse(payload);
      setResponse(body);
      setRunState('success');
      trackMarketingEvent('demo_decision_shown', {
        page: '/demo/procurement',
        location: 'procurement_workflow',
        scenario,
        decision: body.result.decision?.effect ?? 'no_action',
        outcome: body.state.purchaseOrders.length === 1 ? 'submitted' : 'not_submitted',
        label: activePolicyIds.join(','),
      });
    } catch (requestError) {
      setError(
        requestError instanceof Error
          ? requestError.message
          : 'The live procurement workflow failed safely.',
      );
      setRunState('error');
      trackMarketingEvent('demo_decision_shown', {
        page: '/demo/procurement',
        location: 'procurement_workflow',
        scenario,
        decision: 'request_error',
        outcome: 'error',
        label: activePolicyIds.join(','),
      });
    }
  }

  function togglePolicy(policyId: ProcurementPolicyId, enabled: boolean) {
    if (runState === 'running') return;
    setSelectedPolicies((current) => {
      const next = new Set(current);
      if (enabled) next.add(policyId);
      else next.delete(policyId);
      return next;
    });
    setRunState('idle');
    setSubmittedPrompt('');
    setResponse(null);
    setError('');
    trackMarketingEvent('demo_policy_changed', {
      page: '/demo/procurement',
      location: 'procurement_policy_stack',
      label: policyId,
      outcome: enabled ? 'enabled' : 'disabled',
    });
  }

  return (
    <div className={styles['shell']}>
      <section className={styles['chatPanel']} aria-labelledby="procurement-chat-title">
        <div className={styles['panelHeader']}>
          <div>
            <p>Buyer workspace</p>
            <h2 id="procurement-chat-title">Procurement agent</h2>
          </div>
          <span className={styles['liveBadge']}>
            <i aria-hidden="true" /> Live OpenAI
          </span>
        </div>

        <div className={styles['chatBody']} aria-live="polite" aria-busy={runState === 'running'}>
          <div className={styles['assistantMessage']}>
            <span>Procurement agent</span>
            <p>
              I can search a fixed demo catalog and propose one purchase order. TrustLoopGuard
              checks the exact supplier, category, and review tier before anything is submitted.
            </p>
          </div>

          {submittedPrompt !== '' ? (
            <div className={styles['buyerMessage']}>
              <span>Buyer</span>
              <p>{submittedPrompt}</p>
            </div>
          ) : null}

          {runState === 'running' ? (
            <div className={styles['workingStatus']}>
              <i className={styles['spinner']} aria-hidden="true" />
              OpenAI is checking the catalog and proposing tools…
            </div>
          ) : null}

          {response !== null ? (
            <div className={styles['assistantMessage']}>
              <span>Procurement agent</span>
              <p>{response.result.finalMessage}</p>
            </div>
          ) : null}

          {error !== '' ? (
            <div className={styles['errorMessage']} role="alert">
              <strong>Purchase order stopped safely</strong>
              <p>{error}</p>
            </div>
          ) : null}
        </div>

        <form className={styles['composer']} onSubmit={runDemo}>
          <div className={styles['exampleRow']} aria-label="Example procurement requests">
            {EXAMPLES.map((example) => (
              <button
                key={example.label}
                type="button"
                onClick={() => setPrompt(example.prompt)}
                disabled={runState === 'running'}
              >
                {example.label}
              </button>
            ))}
          </div>
          <label htmlFor="procurement-prompt">Buyer request</label>
          <textarea
            id="procurement-prompt"
            value={prompt}
            onChange={(event) => setPrompt(event.target.value)}
            maxLength={500}
            rows={3}
            disabled={runState === 'running'}
          />
          <button className={styles['runButton']} type="submit" disabled={runState === 'running'}>
            {runState === 'running' ? 'Running live agent' : 'Run secure procurement agent'}
            <span aria-hidden="true">→</span>
          </button>
        </form>
      </section>

      <section className={styles['controlPanel']} aria-labelledby="procurement-controls-title">
        <div className={styles['panelHeader']}>
          <div>
            <p>Rust-enforced controls</p>
            <h2 id="procurement-controls-title">TrustLoopGuard policy stack</h2>
          </div>
          <DecisionBadge response={response} runState={runState} />
        </div>

        <div className={styles['policyStack']}>
          {PROCUREMENT_POLICY_IDS.map((policyId) => {
            const policy = POLICY_COPY[policyId];
            const checked = selectedPolicies.has(policyId);
            return (
              <label className={styles['policyCard']} key={policyId}>
                <input
                  type="checkbox"
                  role="switch"
                  checked={checked}
                  onChange={(event) => togglePolicy(policyId, event.currentTarget.checked)}
                  disabled={runState === 'running'}
                  aria-describedby={`${policyId}-description`}
                />
                <span className={styles['switchTrack']} aria-hidden="true">
                  <i />
                </span>
                <span className={styles['policyCopy']}>
                  <strong>{policy.title}</strong>
                  <small id={`${policyId}-description`}>{policy.description}</small>
                </span>
                <b className={styles[policy.effect === 'Review' ? 'reviewEffect' : 'denyEffect']}>
                  {policy.effect}
                </b>
              </label>
            );
          })}
          {activePolicyIds.length === 0 ? (
            <p className={styles['unprotectedNotice']} role="status">
              Unprotected profile selected. TrustLoopGuard will still record the proposed action,
              but these three demo policies will not apply.
            </p>
          ) : null}
        </div>

        <div className={styles['workflow']} aria-live="polite">
          <WorkflowStep
            number="01"
            title="OpenAI agent"
            detail="Searches the catalog and chooses whether to propose submit_purchase_order."
            state={runState === 'idle' ? 'idle' : runState === 'running' ? 'running' : 'complete'}
          />
          <WorkflowStep
            number="02"
            title="Catalog evidence"
            detail={
              traceSummary(response, 'search_catalog') ??
              'Resolves a quote to server-owned supplier and price facts.'
            }
            state={toolStepState(response, runState, 'search_catalog')}
          />
          <WorkflowStep
            number="03"
            title="TrustLoopGuard"
            detail={
              decision?.reason ??
              'Evaluates the exact action against the selected Rust policy profile.'
            }
            state={guardStepState(response, runState)}
            emphasized
          />
          <WorkflowStep
            number="04"
            title="Procurement system"
            detail={providerDetail(response)}
            state={providerStepState(response, runState)}
          />
        </div>

        <div className={styles['proofGrid']}>
          <article>
            <span>Authorization proof</span>
            <strong>{decision?.traceId ?? 'Waiting for a proposed action'}</strong>
            <dl>
              <div>
                <dt>Effect</dt>
                <dd>
                  {decision?.effect ?? (response === null ? 'Not checked' : 'No action proposed')}
                </dd>
              </div>
              <div>
                <dt>Matched policy</dt>
                <dd>{matchedPolicyIds(response)}</dd>
              </div>
              <div>
                <dt>Latency</dt>
                <dd>{decision === undefined ? '—' : `${decision.latencyMs} ms`}</dd>
              </div>
              <div>
                <dt>Approval</dt>
                <dd>{decision?.approvalId ?? 'Not required'}</dd>
              </div>
            </dl>
          </article>

          <article>
            <span>Provider outcome</span>
            <strong>{purchaseOrder?.id ?? 'No purchase order created'}</strong>
            <dl>
              <div>
                <dt>Status</dt>
                <dd>{purchaseOrder?.status ?? 'Not called'}</dd>
              </div>
              <div>
                <dt>Supplier</dt>
                <dd>{purchaseOrder?.supplierName ?? '—'}</dd>
              </div>
              <div>
                <dt>Total</dt>
                <dd>{purchaseOrder === undefined ? '—' : formatMoney(purchaseOrder.totalMinor)}</dd>
              </div>
            </dl>
          </article>
        </div>
      </section>
    </div>
  );
}

function WorkflowStep({
  number,
  title,
  detail,
  state,
  emphasized = false,
}: {
  number: string;
  title: string;
  detail: string;
  state: StepState;
  emphasized?: boolean;
}) {
  return (
    <article
      className={`${styles['workflowStep']} ${styles[state]} ${emphasized ? styles['guardStep'] : ''}`}
    >
      <span>{number}</span>
      <div>
        <h3>{title}</h3>
        <p>{detail}</p>
      </div>
      <i aria-hidden="true" />
    </article>
  );
}

function DecisionBadge({
  response,
  runState,
}: {
  response: ProcurementDemoResponse | null;
  runState: RunState;
}) {
  const effect = response?.result.decision?.effect;
  const label =
    runState === 'running'
      ? 'Checking'
      : effect === 'permit'
        ? 'Permitted'
        : effect === 'require_approval'
          ? 'Held for review'
          : effect === 'deny' || effect === 'defer'
            ? 'Blocked'
            : response === null
              ? 'Ready'
              : 'No action';
  const state =
    effect === 'permit'
      ? 'permitted'
      : effect === 'require_approval'
        ? 'reviewed'
        : effect === 'deny' || effect === 'defer'
          ? 'blocked'
          : 'ready';
  return <span className={`${styles['decisionBadge']} ${styles[state]}`}>{label}</span>;
}

function traceSummary(
  response: ProcurementDemoResponse | null,
  tool: ProcurementDemoResponse['result']['traces'][number]['tool'],
): string | undefined {
  return response?.result.traces.find((trace) => trace.tool === tool)?.summary;
}

function toolStepState(
  response: ProcurementDemoResponse | null,
  runState: RunState,
  tool: ProcurementDemoResponse['result']['traces'][number]['tool'],
): StepState {
  if (traceSummary(response, tool) !== undefined) return 'complete';
  return runState === 'running' ? 'running' : 'idle';
}

function guardStepState(response: ProcurementDemoResponse | null, runState: RunState): StepState {
  if (response?.result.decision !== undefined) return 'complete';
  if (response !== null) return 'stopped';
  return runState === 'running' ? 'running' : 'idle';
}

function providerStepState(
  response: ProcurementDemoResponse | null,
  runState: RunState,
): StepState {
  if (response?.state.purchaseOrders.length === 1) return 'complete';
  if (response !== null) return 'stopped';
  return runState === 'running' ? 'running' : 'idle';
}

function providerDetail(response: ProcurementDemoResponse | null): string {
  const purchaseOrder = response?.state.purchaseOrders[0];
  if (purchaseOrder !== undefined) return `Submitted ${purchaseOrder.id} after a permit decision.`;
  const effect = response?.result.decision?.effect;
  if (effect === 'require_approval')
    return 'Not called. The purchase order is waiting for human approval.';
  if (effect === 'deny' || effect === 'defer')
    return 'Not called. TrustLoopGuard stopped the proposed action.';
  if (response !== null) return 'Not called. The agent did not propose a purchase order.';
  return 'Receives one simulated purchase order only after TrustLoopGuard permits it.';
}

function matchedPolicyIds(response: ProcurementDemoResponse | null): string {
  const policyIds = response?.result.decision?.findings
    .map((finding) => finding.policyId)
    .filter((policyId): policyId is string => policyId !== undefined);
  return policyIds === undefined || policyIds.length === 0 ? 'None' : policyIds.join(', ');
}

function demoScenario(message: string): string {
  return EXAMPLES.find((example) => example.prompt === message)?.label ?? 'custom';
}

function publicError(payload: JsonValue): string | undefined {
  if (payload === null || Array.isArray(payload) || typeof payload !== 'object') return undefined;
  return typeof payload['error'] === 'string' ? payload['error'] : undefined;
}

function formatMoney(amountMinor: number): string {
  return new Intl.NumberFormat('en-US', { style: 'currency', currency: 'USD' }).format(
    amountMinor / 100,
  );
}
