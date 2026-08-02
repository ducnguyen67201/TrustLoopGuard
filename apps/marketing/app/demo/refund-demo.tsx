'use client';

import { useEffect, useState, type FormEvent } from 'react';
import { trackMarketingEvent } from '@/lib/gtm';
import type { MarketingLocale } from '@/lib/marketing-locale';
import type { RefundDemoResponse, RefundDemoStatus } from './contract';
import { DemoMeetingPrompt, useDemoMeetingPrompt } from './demo-meeting-prompt';
import {
  REFUND_UI_COPY,
  type RefundDecision,
} from './refund-content';
import { mergeRefundDemoStatus } from './status-model';
import { refundDemoReviewUrl } from './review-url';
import styles from './demo.module.css';

type RunState = 'idle' | 'running' | 'success' | 'error';

export function RefundDemo({ locale = 'en' }: { locale?: MarketingLocale }) {
  const { isMeetingPromptOpen, recordCompletedInteraction, dismissMeetingPrompt } =
    useDemoMeetingPrompt();
  const copy = REFUND_UI_COPY[locale];
  const [prompt, setPrompt] = useState<string>(copy.examples[1].prompt);
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
    const scenario = demoScenario(message, copy.examples);

    trackMarketingEvent('demo_started', {
      page: copy.pagePath,
      location: 'refund_composer',
      scenario,
    });

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
      if (!result.ok) {
        throw new Error(
          locale === 'vi'
            ? result.status === 429
              ? copy.dailyLimit
              : copy.workflowFailed
            : (body.error ?? copy.workflowFailed),
        );
      }
      setResponse(body);
      setRunState('success');
      recordCompletedInteraction();
      trackMarketingEvent('demo_decision_shown', {
        page: copy.pagePath,
        location: 'refund_workflow',
        scenario,
        decision: analyticsDecision(body),
        outcome: 'success',
      });
    } catch (requestError) {
      setError(
        requestError instanceof Error
          ? requestError.message
          : copy.workflowFailed,
      );
      setRunState('error');
      trackMarketingEvent('demo_decision_shown', {
        page: copy.pagePath,
        location: 'refund_workflow',
        scenario,
        decision: 'request_error',
        outcome: 'error',
      });
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
            <p>{copy.customerSupport}</p>
            <h2 id="refund-chat-title">{copy.chatTitle}</h2>
          </div>
          <span className={styles['liveBadge']}>
            <i aria-hidden="true" /> {copy.live}
          </span>
        </div>

        <div className={styles['chatBody']} aria-live="polite">
          <div className={styles['assistantMessage']}>
            <span>{copy.supportAgent}</span>
            <p>
              {copy.greetingBeforeOrder} <code>ord_demo_1001</code> {copy.greetingAfterOrder}
            </p>
          </div>

          {submittedPrompt !== '' ? (
            <div className={styles['customerMessage']}>
              <span>{copy.customer}</span>
              <p>{submittedPrompt}</p>
            </div>
          ) : null}

          {runState === 'running' ? (
            <div className={styles['agentWorking']} role="status">
              <span className={styles['spinner']} aria-hidden="true" />
              {copy.agentWorking}
            </div>
          ) : null}

          {response !== null ? (
            <div className={styles['assistantMessage']}>
              <span>{copy.supportAgent}</span>
              <p>{response.result.finalMessage}</p>
            </div>
          ) : null}

          {isHeld && actionId !== undefined ? (
            <div className={styles['reviewCallout']}>
              <div>
                <strong>{copy.approvalRequired}</strong>
                <p>{copy.approvalDescription}</p>
              </div>
              <a href={refundDemoReviewUrl(actionId)} target="_blank" rel="noreferrer">
                {copy.reviewAction} <span aria-hidden="true">↗</span>
              </a>
            </div>
          ) : null}

          {error !== '' ? (
            <div className={styles['errorMessage']} role="alert">
              <strong>{copy.errorTitle}</strong>
              <p>{error}</p>
            </div>
          ) : null}
        </div>

        <form className={styles['composer']} onSubmit={runDemo}>
          <div className={styles['exampleRow']} aria-label={copy.examplesLabel}>
            {copy.examples.map((example) => (
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
          <label htmlFor="refund-prompt">{copy.messageLabel}</label>
          <textarea
            id="refund-prompt"
            value={prompt}
            onChange={(event) => setPrompt(event.target.value)}
            maxLength={500}
            rows={2}
          />
          <button className={styles['runButton']} type="submit" disabled={runState === 'running'}>
            {runState === 'running' ? copy.runningWorkflow : copy.runRefund}
            <span aria-hidden="true">→</span>
          </button>
        </form>
      </section>

      <section className={styles['controlPanel']} aria-labelledby="control-boundary-title">
        <div className={styles['panelHeader']}>
          <div>
            <p>{copy.executionTrace}</p>
            <h2 id="control-boundary-title">{copy.controlBoundary}</h2>
          </div>
          <DecisionBadge decision={decision} locale={locale} />
        </div>

        <div className={styles['workflow']}>
          <WorkflowStep
            number="01"
            title={copy.openAiAgent}
            detail={copy.openAiDetail}
            state={runState === 'idle' ? 'idle' : 'complete'}
          />
          <WorkflowStep
            number="02"
            title={copy.orderEvidence}
            detail={localizedTraceDetail(
              traceSummary(response, 'search_order'),
              locale,
              copy.orderEvidenceIdle,
              copy.orderEvidenceComplete,
            )}
            state={traceState(response, runState, 'search_order')}
          />
          <WorkflowStep
            number="03"
            title="Featherlane AI"
            detail={localizedTraceDetail(
              traceSummary(response, 'prepare_refund'),
              locale,
              copy.guardDetail,
              copy.guardComplete[decision],
            )}
            state={traceState(response, runState, 'prepare_refund')}
            emphasized
          />
          <WorkflowStep
            number="04"
            title={copy.stripeTestMode}
            detail={stripeDetail(response, decision, locale)}
            state={stripeState(decision)}
          />
        </div>

        <div className={styles['proofGrid']}>
          <article>
            <span>{copy.seededOrder}</span>
            <strong>{order?.id ?? 'ord_demo_1001'}</strong>
            <dl>
              <div>
                <dt>{copy.captured}</dt>
                <dd>{formatMoney(order?.amountPaidMinor ?? 10_000, locale)}</dd>
              </div>
              <div>
                <dt>{copy.refundable}</dt>
                <dd>{formatMoney(order?.refundableBalanceMinor ?? 10_000, locale)}</dd>
              </div>
              <div>
                <dt>{copy.payment}</dt>
                <dd>Visa ···· {order?.paymentMethodLast4 ?? '4242'}</dd>
              </div>
            </dl>
          </article>

          <article>
            <span>{copy.decisionProof}</span>
            <strong>{response?.result.actionId ?? copy.waitingForAction}</strong>
            <dl>
              <div>
                <dt>{copy.decision}</dt>
                <dd>{decisionLabel(decision, locale)}</dd>
              </div>
              <div>
                <dt>{copy.receipt}</dt>
                <dd>{response?.result.receiptId ?? copy.notCreated}</dd>
              </div>
              <div>
                <dt>{copy.provider}</dt>
                <dd>{latestRefund?.providerReference ?? copy.notCalled}</dd>
              </div>
            </dl>
          </article>
        </div>
      </section>
      <DemoMeetingPrompt
        open={isMeetingPromptOpen}
        onClose={dismissMeetingPrompt}
        page={copy.pagePath}
        locale={locale}
      />
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

function DecisionBadge({
  decision,
  locale,
}: {
  decision: RefundDecision;
  locale: MarketingLocale;
}) {
  return (
    <span className={`${styles['decisionBadge']} ${styles[decision]}`}>
      {decisionLabel(decision, locale)}
    </span>
  );
}

function decisionFrom(
  response: RefundDemoResponse | null,
  runState: RunState,
): RefundDecision {
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

function stripeState(decision: RefundDecision): 'idle' | 'running' | 'complete' | 'stopped' {
  if (decision === 'running') return 'running';
  if (decision === 'executed') return 'complete';
  if (decision === 'held' || decision === 'blocked' || decision === 'checked') return 'stopped';
  return 'idle';
}

function stripeDetail(
  response: RefundDemoResponse | null,
  decision: RefundDecision,
  locale: MarketingLocale,
): string {
  const copy = REFUND_UI_COPY[locale];
  const execution = traceSummary(response, 'execute_refund');
  if (execution !== undefined) return locale === 'vi' ? copy.stripeExecuted : execution;
  if (decision === 'held') return copy.stripeHeld;
  if (decision === 'blocked') return copy.stripeBlocked;
  return copy.stripeDefault;
}

function decisionLabel(decision: RefundDecision, locale: MarketingLocale): string {
  return REFUND_UI_COPY[locale].decisionLabels[decision];
}

function localizedTraceDetail(
  trace: string | undefined,
  locale: MarketingLocale,
  idle: string,
  localizedComplete: string,
): string {
  if (trace === undefined) return idle;
  return locale === 'vi' ? localizedComplete : trace;
}

function demoScenario(
  message: string,
  examples: readonly { label: string; prompt: string }[],
): string {
  return examples.find((example) => example.prompt === message)?.label ?? 'custom';
}

function analyticsDecision(response: RefundDemoResponse): string {
  const decision = decisionFrom(response, 'success');
  const labels: Record<RefundDecision, string> = {
    ready: 'ready',
    running: 'running',
    executed: 'permit',
    held: 'require_approval',
    blocked: 'deny',
    checked: 'checked',
  };
  return labels[decision];
}

function formatMoney(amountMinor: number, locale: MarketingLocale): string {
  return new Intl.NumberFormat(locale === 'vi' ? 'vi-VN' : 'en-US', {
    style: 'currency',
    currency: 'USD',
  }).format(
    amountMinor / 100,
  );
}
