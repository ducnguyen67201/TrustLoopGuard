'use client';

import { useEffect, useMemo, useState, type FormEvent } from 'react';

import { trackMarketingEvent } from '@/lib/gtm';
import type { MarketingLocale } from '@/lib/marketing-locale';

import {
  PROCUREMENT_POLICY_IDS,
  sanitizeProcurementDemoResponse,
  sanitizeProcurementPolicyInventory,
  type JsonValue,
  type ProcurementDemoResponse,
  type ProcurementPolicy,
  type ProcurementPolicyInventory,
} from './contract';
import { PROCUREMENT_DEMO_COPY } from './content';
import styles from './procurement.module.css';

type RunState = 'idle' | 'running' | 'success' | 'error';
type StepState = 'idle' | 'running' | 'complete' | 'stopped';
type InventoryState = 'loading' | 'ready' | 'error';

type ProcurementDemoPresentation = {
  companyName: string;
  workflow: string;
};

export function ProcurementDemo({
  locale = 'en',
  presentation,
}: {
  locale?: MarketingLocale;
  presentation?: ProcurementDemoPresentation;
}) {
  const copy = PROCUREMENT_DEMO_COPY[locale];
  const analyticsPage = locale === 'vi' ? '/vi/demo/procurement' : '/demo/procurement';
  const [prompt, setPrompt] = useState<string>(copy.examples[1].prompt);
  const [submittedPrompt, setSubmittedPrompt] = useState('');
  const [runState, setRunState] = useState<RunState>('idle');
  const [response, setResponse] = useState<ProcurementDemoResponse | null>(null);
  const [policies, setPolicies] = useState<ProcurementPolicy[]>([]);
  const [inventoryState, setInventoryState] = useState<InventoryState>('loading');
  const [inventorySource, setInventorySource] = useState<
    ProcurementPolicyInventory['source'] | null
  >(null);
  const [workspace, setWorkspace] = useState<
    ProcurementPolicyInventory['workspace'] | null
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
        setWorkspace(inventory.workspace);
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
    const scenario = demoScenario(message, copy.examples);

    trackMarketingEvent('demo_started', {
      page: analyticsPage,
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
        throw new Error(
          locale === 'vi'
            ? result.status === 429
              ? copy.limitError
              : copy.fallbackError
            : (publicError(payload) ?? copy.fallbackError),
        );
      }
      const body = sanitizeProcurementDemoResponse(payload);
      setResponse(body);
      setRunState('success');
      trackMarketingEvent('demo_decision_shown', {
        page: analyticsPage,
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
          : copy.fallbackError,
      );
      setRunState('error');
      trackMarketingEvent('demo_decision_shown', {
        page: analyticsPage,
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
            <p>
              {presentation ? `Prepared for ${presentation.companyName}` : copy.buyerWorkspace}
            </p>
            <h2 id="procurement-chat-title">
              {presentation?.workflow ?? copy.agentTitle}
            </h2>
          </div>
          <span className={styles['liveBadge']}>
            <i aria-hidden="true" /> {copy.liveOpenAi}
          </span>
        </div>

        <div className={styles['chatBody']} aria-live="polite" aria-busy={runState === 'running'}>
          <div className={styles['assistantMessage']}>
            <span>{copy.agentLabel}</span>
            <p>{copy.greeting}</p>
          </div>

          {submittedPrompt !== '' ? (
            <div className={styles['buyerMessage']}>
              <span>{copy.buyerLabel}</span>
              <p>{submittedPrompt}</p>
            </div>
          ) : null}

          {runState === 'running' ? (
            <div className={styles['workingStatus']}>
              <i className={styles['spinner']} aria-hidden="true" />
              {copy.working}
            </div>
          ) : null}

          {response !== null ? (
            <div className={styles['assistantMessage']}>
              <span>{copy.agentLabel}</span>
              <p>{response.result.finalMessage}</p>
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
          <label htmlFor="procurement-prompt">{copy.buyerRequest}</label>
          <textarea
            id="procurement-prompt"
            value={prompt}
            onChange={(event) => setPrompt(event.target.value)}
            maxLength={500}
            rows={3}
            disabled={runState === 'running'}
          />
          <button className={styles['runButton']} type="submit" disabled={runState === 'running'}>
            {runState === 'running' ? copy.runningButton : copy.runButton}
            <span aria-hidden="true">→</span>
          </button>
        </form>
      </section>

      <section className={styles['controlPanel']} aria-labelledby="procurement-controls-title">
        <div className={styles['panelHeader']}>
          <div>
            <p>{copy.controlsEyebrow}</p>
            <h2 id="procurement-controls-title">{copy.controlsTitle}</h2>
          </div>
          <DecisionBadge response={response} runState={runState} locale={locale} />
        </div>

        <div className={styles['workflow']} aria-live="polite">
          <WorkflowStep
            number="01"
            title={copy.workflow[0].title}
            detail={
              response === null ? copy.workflow[0].idle : copy.workflow[0].complete
            }
            state={runState === 'idle' ? 'idle' : runState === 'running' ? 'running' : 'complete'}
          />
          <WorkflowStep
            number="02"
            title={copy.workflow[1].title}
            detail={catalogDetail(response, locale)}
            state={toolStepState(response, runState, 'search_catalog')}
          />
          <WorkflowStep
            number="03"
            title={copy.workflow[2].title}
            detail={guardDetail(response, locale)}
            state={guardStepState(response, runState)}
            emphasized
          />
          <WorkflowStep
            number="04"
            title={copy.workflow[3].title}
            detail={providerDetail(response, locale)}
            state={providerStepState(response, runState)}
          />
        </div>

        <section className={styles['policyInventory']} aria-labelledby="policy-checks-title">
          <div className={styles['monitorSectionHeading']}>
            <h3 id="policy-checks-title">{copy.policiesChecked}</h3>
            <span aria-live="polite">
              {policyMonitorSummary(policies, inventoryState, inventorySource, locale)}
            </span>
          </div>
          {inventoryState === 'loading' ? (
            <p className={styles['inventoryNotice']} role="status">
              {copy.loadingRegistry}
            </p>
          ) : null}
          {inventoryState === 'error' ? (
            <p className={styles['inventoryNotice']} role="status">
              {copy.inventoryUnavailable}
            </p>
          ) : null}
          {inventoryState === 'ready' && inventorySource === 'demo_template' ? (
            <p className={styles['inventoryNotice']} role="status">
              <strong>{copy.previewLead}</strong> {copy.previewDetail}
            </p>
          ) : null}
          {inventoryState === 'ready' && inventorySource === 'rust' && policies.length === 0 ? (
            <p className={styles['inventoryNotice']}>
              {copy.noPolicies}
            </p>
          ) : null}
          {inventoryState === 'ready' && workspace !== null ? (
            <div className={styles['workspaceContext']}>
              <span>
                {inventorySource === 'rust'
                  ? copy.policyWorkspace
                  : copy.previewWorkspace}
              </span>
              <code>
                {workspace.source === 'configured'
                  ? workspace.id
                  : copy.serverDefaultWorkspace}
              </code>
            </div>
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
                    <strong>
                      {locale === 'vi'
                        ? copy.policyDescriptions[policy.id]
                        : (policy.description ?? policy.id)}
                    </strong>
                    <code>{policy.id}</code>
                    <span className={styles['policyStatus']}>
                      {matched
                        ? copy.matchedThisAction
                        : policy.enabled
                          ? copy.activeInRust
                          : copy.policyPackPreview}
                    </span>
                  </div>
                  <div className={styles['policyMeta']}>
                    <span>{copy.action}</span>
                    <span>{policySeverityLabel(policy.severity, locale)}</span>
                    <span>{policyActionLabel(policy.action, locale)}</span>
                  </div>
                </article>
              );
            })}
          </div>
        </section>

        <div className={styles['proofGrid']}>
          <article>
            <span>{copy.authorizationProof}</span>
            <strong>{decision?.traceId ?? copy.waitingForAction}</strong>
            <dl>
              <div>
                <dt>{copy.effect}</dt>
                <dd>
                  {decision === undefined
                    ? response === null
                      ? copy.notChecked
                      : copy.noAction
                    : decisionEffectLabel(decision.effect, locale)}
                </dd>
              </div>
              <div>
                <dt>{copy.matchedPolicy}</dt>
                <dd>{matchedPolicyIds(response, locale)}</dd>
              </div>
              <div>
                <dt>{copy.latency}</dt>
                <dd>{decision === undefined ? '—' : `${decision.latencyMs} ms`}</dd>
              </div>
              <div>
                <dt>{copy.approval}</dt>
                <dd>{decision?.approvalId ?? copy.notRequired}</dd>
              </div>
            </dl>
          </article>

          <article>
            <span>{copy.providerOutcome}</span>
            <strong>{purchaseOrder?.id ?? copy.noPurchaseOrder}</strong>
            <dl>
              <div>
                <dt>{copy.status}</dt>
                <dd>{purchaseOrder === undefined ? copy.notCalled : copy.submitted}</dd>
              </div>
              <div>
                <dt>{copy.supplier}</dt>
                <dd>{purchaseOrder?.supplierName ?? '—'}</dd>
              </div>
              <div>
                <dt>{copy.total}</dt>
                <dd>
                  {purchaseOrder === undefined
                    ? '—'
                    : formatMoney(purchaseOrder.totalMinor, locale)}
                </dd>
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
  locale,
}: {
  response: ProcurementDemoResponse | null;
  runState: RunState;
  locale: MarketingLocale;
}) {
  const copy = PROCUREMENT_DEMO_COPY[locale];
  const effect = response?.result.decision?.effect;
  const label =
    runState === 'running'
      ? copy.checking
      : effect === 'permit'
        ? copy.permitted
        : effect === 'require_approval'
          ? copy.heldForReview
          : effect === 'deny' || effect === 'defer'
            ? copy.blocked
            : response === null
              ? copy.ready
              : copy.noAction;
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

function guardDetail(
  response: ProcurementDemoResponse | null,
  locale: MarketingLocale,
): string {
  const copy = PROCUREMENT_DEMO_COPY[locale].workflow[2];
  if (locale === 'en' && response?.result.decision !== undefined) {
    return response.result.decision.reason;
  }
  const effect = response?.result.decision?.effect;
  if (effect === 'permit' || effect === 'transform') return copy.permit;
  if (effect === 'require_approval') return copy.review;
  if (effect === 'deny' || effect === 'defer') return copy.deny;
  if (response !== null) return copy.noAction;
  return copy.idle;
}

function providerDetail(
  response: ProcurementDemoResponse | null,
  locale: MarketingLocale,
): string {
  const copy = PROCUREMENT_DEMO_COPY[locale].workflow[3];
  const purchaseOrder = response?.state.purchaseOrders[0];
  if (purchaseOrder !== undefined) {
    return locale === 'en'
      ? `Submitted ${purchaseOrder.id} after a permit decision.`
      : copy.submitted;
  }
  const effect = response?.result.decision?.effect;
  if (effect === 'require_approval') return copy.review;
  if (effect === 'deny' || effect === 'defer') return copy.blocked;
  if (response !== null) return copy.noAction;
  return copy.idle;
}

function catalogDetail(
  response: ProcurementDemoResponse | null,
  locale: MarketingLocale,
): string {
  const copy = PROCUREMENT_DEMO_COPY[locale].workflow[1];
  const summary = traceSummary(response, 'search_catalog');
  if (summary === undefined) return copy.idle;
  return locale === 'en' ? summary : copy.complete;
}

function matchedPolicyIds(
  response: ProcurementDemoResponse | null,
  locale: MarketingLocale,
): string {
  const policyIds = response?.result.decision?.findings
    .map((finding) => finding.policyId)
    .filter((policyId): policyId is string => policyId !== undefined);
  return policyIds === undefined || policyIds.length === 0
    ? PROCUREMENT_DEMO_COPY[locale].none
    : policyIds.join(', ');
}

function policyMonitorSummary(
  policies: ProcurementPolicy[],
  inventoryState: InventoryState,
  inventorySource: ProcurementPolicyInventory['source'] | null,
  locale: MarketingLocale,
): string {
  const copy = PROCUREMENT_DEMO_COPY[locale];
  if (inventoryState === 'loading') return copy.loading;
  if (inventoryState === 'error') return copy.unavailable;
  return `${policies.length} ${inventorySource === 'rust' ? copy.active : copy.inPack}`;
}

function policyActionLabel(action: string | undefined, locale: MarketingLocale): string {
  if (locale === 'vi') {
    if (action === 'deny') return 'từ chối';
    if (action === 'require_approval') return 'cần phê duyệt';
    if (action === 'permit') return 'cho phép';
    if (action === 'transform') return 'chuyển đổi';
  }
  return action?.replaceAll('_', ' ') ?? PROCUREMENT_DEMO_COPY[locale].check;
}

function policySeverityLabel(
  severity: ProcurementPolicy['severity'],
  locale: MarketingLocale,
): string {
  if (locale === 'en') return severity;
  if (severity === 'critical') return 'nghiêm trọng';
  if (severity === 'high') return 'cao';
  if (severity === 'medium') return 'trung bình';
  return 'thấp';
}

function decisionEffectLabel(
  effect: NonNullable<ProcurementDemoResponse['result']['decision']>['effect'],
  locale: MarketingLocale,
): string {
  if (locale === 'en') return effect;
  if (effect === 'permit') return 'cho phép';
  if (effect === 'require_approval') return 'cần phê duyệt';
  if (effect === 'deny') return 'từ chối';
  if (effect === 'transform') return 'chuyển đổi';
  return 'tạm hoãn';
}

function demoScenario(
  message: string,
  examples: readonly { label: string; prompt: string }[],
): string {
  return examples.find((example) => example.prompt === message)?.label ?? 'custom';
}

function publicError(payload: JsonValue): string | undefined {
  if (payload === null || Array.isArray(payload) || typeof payload !== 'object') return undefined;
  return typeof payload['error'] === 'string' ? payload['error'] : undefined;
}

function formatMoney(amountMinor: number, locale: MarketingLocale): string {
  return new Intl.NumberFormat(locale === 'vi' ? 'vi-VN' : 'en-US', {
    style: 'currency',
    currency: 'USD',
  }).format(
    amountMinor / 100,
  );
}
