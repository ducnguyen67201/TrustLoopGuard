'use client';

import { useEffect, useMemo, useState, type FormEvent } from 'react';

import { trackMarketingEvent } from '@/lib/gtm';

import {
  PROCUREMENT_POLICY_IDS,
  sanitizeProcurementDemoResponse,
  sanitizeProcurementPolicyInventory,
  type JsonValue,
  type ProcurementDemoResponse,
  type ProcurementPolicy,
  type ProcurementPolicyInventory,
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

type RunState = 'idle' | 'running' | 'success' | 'error';
type StepState = 'idle' | 'running' | 'complete' | 'stopped';
type InventoryState = 'loading' | 'ready' | 'error';

export function ProcurementDemo() {
  const [prompt, setPrompt] = useState<string>(EXAMPLES[1].prompt);
  const [submittedPrompt, setSubmittedPrompt] = useState('');
  const [runState, setRunState] = useState<RunState>('idle');
  const [response, setResponse] = useState<ProcurementDemoResponse | null>(null);
  const [policies, setPolicies] = useState<ProcurementPolicy[]>([]);
  const [inventoryState, setInventoryState] = useState<InventoryState>('loading');
  const [inventorySource, setInventorySource] = useState<
    ProcurementPolicyInventory['source'] | null
  >(null);
  const [error, setError] = useState('');

  useEffect(() => {
    let active = true;
    async function loadPolicies(): Promise<void> {
      try {
        const result = await fetch('/api/demo/procurement', { cache: 'no-store' });
        if (!result.ok) throw new Error('Policy inventory request failed');
        const inventory = sanitizeProcurementPolicyInventory(await result.json());
        if (!active) return;
        setPolicies(inventory.policies);
        setInventorySource(inventory.source);
        setInventoryState('ready');
      } catch {
        if (active) setInventoryState('error');
      }
    }
    void loadPolicies();
    return () => {
      active = false;
    };
  }, []);

  const matchedPolicyIdSet = useMemo(
    () =>
      new Set(
        response?.result.decision?.findings.flatMap((finding) =>
          finding.policyId === undefined ? [] : [finding.policyId],
        ) ?? [],
      ),
    [response],
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
      label: PROCUREMENT_POLICY_IDS.join(','),
    });

    setRunState('running');
    setSubmittedPrompt(message);
    setResponse(null);
    setError('');

    try {
      const result = await fetch('/api/demo/procurement', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ prompt: message, activePolicyIds: PROCUREMENT_POLICY_IDS }),
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
        label: PROCUREMENT_POLICY_IDS.join(','),
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
        label: PROCUREMENT_POLICY_IDS.join(','),
      });
    }
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
              'Evaluates the exact action against the enabled Rust policy profile.'
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

        <section className={styles['policyInventory']} aria-labelledby="policy-checks-title">
          <div className={styles['monitorSectionHeading']}>
            <h3 id="policy-checks-title">Policies checked</h3>
            <span aria-live="polite">
              {policyMonitorSummary(policies, inventoryState, inventorySource)}
            </span>
          </div>
          {inventoryState === 'loading' ? (
            <p className={styles['inventoryNotice']} role="status">
              Loading the policy registry…
            </p>
          ) : null}
          {inventoryState === 'error' ? (
            <p className={styles['inventoryNotice']} role="status">
              Policy inventory unavailable. Purchase actions still fail closed.
            </p>
          ) : null}
          {inventoryState === 'ready' && inventorySource === 'demo_template' ? (
            <p className={styles['inventoryNotice']} role="status">
              <strong>Policy pack preview.</strong> The Rust registry is unavailable, so these are
              the action policies installed by the demo setup. Runtime checks still fail closed.
            </p>
          ) : null}
          {inventoryState === 'ready' && inventorySource === 'rust' && policies.length === 0 ? (
            <p className={styles['inventoryNotice']}>
              No enabled procurement demo policies were found. Run the demo setup command.
            </p>
          ) : null}
          <div className={styles['policyList']}>
            {policies.map((policy) => {
              const matched = matchedPolicyIdSet.has(policy.id);
              return (
                <article
                  key={policy.id}
                  className={`${styles['policyCard']} ${
                    matched ? styles['matchedPolicy'] : ''
                  } ${policy.enabled ? '' : styles['previewPolicy']}`}
                >
                  <span className={styles['policyDot']} aria-hidden="true" />
                  <div>
                    <strong>{policy.description ?? policy.id}</strong>
                    <code>{policy.id}</code>
                    <span className={styles['policyStatus']}>
                      {matched
                        ? 'Matched this action'
                        : policy.enabled
                          ? 'Active in Rust'
                          : 'Policy pack preview'}
                    </span>
                  </div>
                  <div className={styles['policyMeta']}>
                    <span>Action</span>
                    <span>{policy.severity}</span>
                    <span>{policyActionLabel(policy.action)}</span>
                  </div>
                </article>
              );
            })}
          </div>
        </section>

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
      className={`${styles['workflowStep']} ${state === 'idle' ? '' : styles[state]} ${
        emphasized ? styles['guardStep'] : ''
      }`}
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

function policyMonitorSummary(
  policies: ProcurementPolicy[],
  inventoryState: InventoryState,
  inventorySource: ProcurementPolicyInventory['source'] | null,
): string {
  if (inventoryState === 'loading') return 'Loading';
  if (inventoryState === 'error') return 'Unavailable';
  return `${policies.length} ${inventorySource === 'rust' ? 'active' : 'in pack'}`;
}

function policyActionLabel(action?: string): string {
  return action?.replaceAll('_', ' ') ?? 'check';
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
