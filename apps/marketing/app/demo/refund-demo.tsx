'use client';

import { useEffect, useState, type FormEvent } from 'react';
import type { RefundDemoResponse, RefundDemoStatus } from './contract';
import { mergeRefundDemoStatus } from './status-model';
import { refundDemoReviewUrl } from './review-url';
import styles from './demo.module.css';

const EXAMPLES = [
  {
    label: '$25 · auto-allow',
    prompt: 'Refund order ord_demo_1001 for $25 because the item arrived damaged.',
  },
  {
    label: '$75 · hold',
    prompt: 'Refund order ord_demo_1001 for $75 because the item arrived damaged.',
  },
  {
    label: '$125 · block',
    prompt: 'Refund order ord_demo_1001 for $125 because the item arrived damaged.',
  },
] as const;

type RunState = 'idle' | 'running' | 'success' | 'error';
type Decision = 'ready' | 'running' | 'executed' | 'held' | 'blocked' | 'checked';

export function RefundDemo() {
  const [prompt, setPrompt] = useState<string>(EXAMPLES[1].prompt);
  const [submittedPrompt, setSubmittedPrompt] = useState('');
  const [runState, setRunState] = useState<RunState>('idle');
  const [response, setResponse] = useState<RefundDemoResponse | null>(null);
  const [error, setError] = useState('');
  const actionId = response?.result.actionId;
  const isHeld = response !== null && decisionFrom(response, runState) === 'held';

  useEffect(() => {
    if (actionId === undefined || !isHeld) return;
    const polledActionId = actionId;
    let active = true;
    let timer: ReturnType<typeof setTimeout> | undefined;

    async function pollStatus() {
      try {
        const result = await fetch(
          `/api/demo/refund?actionId=${encodeURIComponent(polledActionId)}`,
          { cache: 'no-store' },
        );
        if (result.ok) {
          const status = (await result.json()) as RefundDemoStatus;
          if (!active) return;
          setResponse((current) =>
            current === null ? current : mergeRefundDemoStatus(current, status),
          );
          if (
            status.authorizationEffect !== 'require_approval' &&
            status.executionStatus !== 'not_started' &&
            status.executionStatus !== 'executing'
          ) return;
        }
      } catch {
        // A transient status failure must not change or falsely complete the held refund.
      }
      if (active) timer = setTimeout(pollStatus, 1_500);
    }

    timer = setTimeout(pollStatus, 1_000);
    return () => {
      active = false;
      if (timer !== undefined) clearTimeout(timer);
    };
  }, [actionId, isHeld]);

  async function runDemo(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const message = prompt.trim();
    if (message === '' || runState === 'running') return;

    setRunState('running');
    setSubmittedPrompt(message);
    setResponse(null);
    setError('');

    try {
      const result = await fetch('/api/demo/refund', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ prompt: message }),
      });
      const body = (await result.json().catch(() => ({}))) as RefundDemoResponse & {
        error?: string;
      };
      if (!result.ok) throw new Error(body.error ?? 'The live refund workflow failed safely.');
      setResponse(body);
      setRunState('success');
    } catch (requestError) {
      setError(
        requestError instanceof Error
          ? requestError.message
          : 'The live refund workflow failed safely.',
      );
      setRunState('error');
    }
  }

  const decision = decisionFrom(response, runState);
  const order = response?.state.orders[0];
  const latestRefund = response?.state.refunds[0];

  return (
    <div className={styles['shell']}>
      <section className={styles['chatPanel']} aria-labelledby="refund-chat-title">
        <div className={styles['panelHeader']}>
          <div>
            <p>Customer support</p>
            <h2 id="refund-chat-title">Ask the refund agent</h2>
          </div>
          <span className={styles['liveBadge']}>
            <i aria-hidden="true" /> Live
          </span>
        </div>

        <div className={styles['chatBody']} aria-live="polite">
          <div className={styles['assistantMessage']}>
            <span>Support agent</span>
            <p>
              I can look up order <code>ord_demo_1001</code> and propose a refund. Every refund
              must pass TrustLoopGuard before Stripe can execute it.
            </p>
          </div>

          {submittedPrompt !== '' ? (
            <div className={styles['customerMessage']}>
              <span>Customer</span>
              <p>{submittedPrompt}</p>
            </div>
          ) : null}

          {runState === 'running' ? (
            <div className={styles['agentWorking']} role="status">
              <span className={styles['spinner']} aria-hidden="true" />
              OpenAI is choosing and calling the refund tools…
            </div>
          ) : null}

          {response !== null ? (
            <div className={styles['assistantMessage']}>
              <span>Support agent</span>
              <p>{response.result.finalMessage}</p>
            </div>
          ) : null}

          {isHeld && actionId !== undefined ? (
            <div className={styles['reviewCallout']}>
              <div>
                <strong>Human approval required</strong>
                <p>Open the exact held action in TrustLoopGuard. This demo updates automatically.</p>
              </div>
              <a href={refundDemoReviewUrl(actionId)} target="_blank" rel="noreferrer">
                Review this exact action <span aria-hidden="true">↗</span>
              </a>
            </div>
          ) : null}

          {error !== '' ? (
            <div className={styles['errorMessage']} role="alert">
              <strong>Refund stopped safely</strong>
              <p>{error}</p>
            </div>
          ) : null}
        </div>

        <form className={styles['composer']} onSubmit={runDemo}>
          <div className={styles['exampleRow']} aria-label="Example refund requests">
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
          <label htmlFor="refund-prompt">Customer message</label>
          <textarea
            id="refund-prompt"
            value={prompt}
            onChange={(event) => setPrompt(event.target.value)}
            maxLength={500}
            rows={3}
          />
          <button className={styles['runButton']} type="submit" disabled={runState === 'running'}>
            {runState === 'running' ? 'Running live workflow' : 'Run live refund'}
            <span aria-hidden="true">→</span>
          </button>
        </form>
      </section>

      <section className={styles['controlPanel']} aria-labelledby="control-boundary-title">
        <div className={styles['panelHeader']}>
          <div>
            <p>Execution trace</p>
            <h2 id="control-boundary-title">The control boundary</h2>
          </div>
          <DecisionBadge decision={decision} />
        </div>

        <div className={styles['workflow']}>
          <WorkflowStep
            number="01"
            title="OpenAI agent"
            detail="Chooses search_order, prepare_refund, and execute_refund tools."
            state={runState === 'idle' ? 'idle' : 'complete'}
          />
          <WorkflowStep
            number="02"
            title="Order evidence"
            detail={traceSummary(response, 'search_order') ?? 'Checks the captured order and refundable balance.'}
            state={traceState(response, runState, 'search_order')}
          />
          <WorkflowStep
            number="03"
            title="TrustLoopGuard"
            detail={
              traceSummary(response, 'prepare_refund') ??
              'Evaluates amount, grant scope, eligibility evidence, and policy.'
            }
            state={traceState(response, runState, 'prepare_refund')}
            emphasized
          />
          <WorkflowStep
            number="04"
            title="Stripe test mode"
            detail={stripeDetail(response, decision)}
            state={stripeState(decision)}
          />
        </div>

        <div className={styles['proofGrid']}>
          <article>
            <span>Seeded order</span>
            <strong>{order?.id ?? 'ord_demo_1001'}</strong>
            <dl>
              <div>
                <dt>Captured</dt>
                <dd>{formatMoney(order?.amountPaidMinor ?? 10_000)}</dd>
              </div>
              <div>
                <dt>Refundable</dt>
                <dd>{formatMoney(order?.refundableBalanceMinor ?? 10_000)}</dd>
              </div>
              <div>
                <dt>Payment</dt>
                <dd>Visa ···· {order?.paymentMethodLast4 ?? '4242'}</dd>
              </div>
            </dl>
          </article>

          <article>
            <span>Decision proof</span>
            <strong>{response?.result.actionId ?? 'Waiting for a proposed action'}</strong>
            <dl>
              <div>
                <dt>Decision</dt>
                <dd>{decisionLabel(decision)}</dd>
              </div>
              <div>
                <dt>Receipt</dt>
                <dd>{response?.result.receiptId ?? 'Not created'}</dd>
              </div>
              <div>
                <dt>Provider</dt>
                <dd>{latestRefund?.providerReference ?? 'Not called'}</dd>
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
  state: 'idle' | 'running' | 'complete' | 'stopped';
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

function DecisionBadge({ decision }: { decision: Decision }) {
  return (
    <span className={`${styles['decisionBadge']} ${styles[decision]}`}>
      {decisionLabel(decision)}
    </span>
  );
}

function decisionFrom(response: RefundDemoResponse | null, runState: RunState): Decision {
  if (runState === 'running') return 'running';
  if (response === null) return 'ready';
  if (response.result.receiptId !== undefined) return 'executed';
  const authorization = traceSummary(response, 'prepare_refund')?.toLowerCase() ?? '';
  if (authorization.includes('require_approval')) return 'held';
  if (authorization.includes('deny')) return 'blocked';
  return 'checked';
}

function traceSummary(
  response: RefundDemoResponse | null,
  tool: RefundDemoResponse['result']['traces'][number]['tool'],
): string | undefined {
  return response?.result.traces.find((trace) => trace.tool === tool)?.summary;
}

function traceState(
  response: RefundDemoResponse | null,
  runState: RunState,
  tool: RefundDemoResponse['result']['traces'][number]['tool'],
): 'idle' | 'running' | 'complete' {
  if (traceSummary(response, tool) !== undefined) return 'complete';
  return runState === 'running' ? 'running' : 'idle';
}

function stripeState(decision: Decision): 'idle' | 'running' | 'complete' | 'stopped' {
  if (decision === 'running') return 'running';
  if (decision === 'executed') return 'complete';
  if (decision === 'held' || decision === 'blocked' || decision === 'checked') return 'stopped';
  return 'idle';
}

function stripeDetail(response: RefundDemoResponse | null, decision: Decision): string {
  const execution = traceSummary(response, 'execute_refund');
  if (execution !== undefined) return execution;
  if (decision === 'held') return 'Not called. The refund is waiting for human approval.';
  if (decision === 'blocked') return 'Not called. TrustLoopGuard blocked the proposed refund.';
  return 'Creates the refund only after TrustLoopGuard authorization.';
}

function decisionLabel(decision: Decision): string {
  const labels: Record<Decision, string> = {
    ready: 'Ready',
    running: 'Checking',
    executed: 'Executed',
    held: 'Held',
    blocked: 'Blocked',
    checked: 'Checked',
  };
  return labels[decision];
}

function formatMoney(amountMinor: number): string {
  return new Intl.NumberFormat('en-US', { style: 'currency', currency: 'USD' }).format(
    amountMinor / 100,
  );
}
