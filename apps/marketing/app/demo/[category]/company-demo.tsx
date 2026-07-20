'use client';

import { useEffect, useMemo, useState, type CSSProperties, type FormEvent } from 'react';

import { trackMarketingEvent } from '@/lib/gtm';
import type { CompanyDemoViewModel } from '../company-profile';
import { CONTEXTUAL_PAGE_COPY, CONTEXTUAL_UI_COPY } from '../contextual-content';
import {
  sanitizeContextualDemoResponse,
  sanitizeContextualPolicyInventory,
  type ContextualDemoRequest,
  type ContextualDemoResponse,
  type ContextualPolicy,
} from '../contextual-contract';
import styles from '../demo.module.css';
import type { HealthcareDemoLocale } from '../healthcare/content';

type CompanyDemoProps = {
  profile: CompanyDemoViewModel;
  locale?: HealthcareDemoLocale;
  pagePath?: string;
};

type CompanyBrandStyle = CSSProperties & {
  '--company-accent': string;
  '--company-accent-soft': string;
};

type RunState =
  | 'idle'
  | 'checking_input'
  | 'generating'
  | 'checking_output'
  | 'success'
  | 'error';
type InventoryState = 'loading' | 'ready' | 'error';
type ContextualEffect = NonNullable<
  ContextualDemoResponse['checks'][number]['effect']
>;

interface DisplayMessage {
  id: string;
  role: 'user' | 'assistant';
  content: string;
}

export function CompanyDemo({
  profile,
  locale = 'en',
  pagePath = `/demo/${profile.slug}`,
}: CompanyDemoProps) {
  const copy = CONTEXTUAL_UI_COPY[locale];
  const endpoint = `/api/demo/contextual/${encodeURIComponent(profile.slug)}`;
  const [sessionId] = useState(() => crypto.randomUUID());
  const [message, setMessage] = useState(profile.paths[0].proposal);
  const [selectedPreset, setSelectedPreset] = useState<
    CompanyDemoViewModel['paths'][number]['effect'] | undefined
  >(profile.paths[0].effect);
  const [messages, setMessages] = useState<DisplayMessage[]>([
    {
      id: 'greeting',
      role: 'assistant',
      content: copy.greeting(profile.workflow),
    },
  ]);
  const [runState, setRunState] = useState<RunState>('idle');
  const [response, setResponse] = useState<ContextualDemoResponse | null>(null);
  const [policies, setPolicies] = useState<ContextualPolicy[]>([]);
  const [inventoryState, setInventoryState] = useState<InventoryState>('loading');
  const [error, setError] = useState('');
  const brandStyle: CompanyBrandStyle = {
    '--company-accent': profile.branding.primary_color,
    '--company-accent-soft': profile.branding.secondary_color,
  };

  useEffect(() => {
    let active = true;
    async function loadPolicies(): Promise<void> {
      try {
        const result = await fetch(endpoint, { cache: 'no-store' });
        if (!result.ok) throw new Error(copy.inventoryRequestFailed);
        const inventory = sanitizeContextualPolicyInventory(await result.json());
        if (!active) return;
        setPolicies(inventory.policies);
        setInventoryState('ready');
      } catch {
        if (active) setInventoryState('error');
      }
    }
    void loadPolicies();
    return () => {
      active = false;
    };
  }, [copy.inventoryRequestFailed, endpoint]);

  const matchedPolicyIds = useMemo(
    () =>
      new Set(
        response?.checks.flatMap((check) =>
          check.findings.flatMap((finding) =>
            finding.policyId === undefined ? [] : [finding.policyId],
          ),
        ) ?? [],
      ),
    [response],
  );

  async function runDemo(event: FormEvent<HTMLFormElement>): Promise<void> {
    event.preventDefault();
    const submittedMessage = message.trim();
    if (submittedMessage === '' || isRunning(runState)) return;
    const history: ContextualDemoRequest['history'] = messages.slice(-8).map((entry) => ({
      role: entry.role,
      content: entry.content,
    }));

    trackMarketingEvent('contextual_demo_started', {
      page: pagePath,
      location: 'contextual_composer',
      scenario: selectedPreset ?? 'custom',
    });
    setMessages((current) => [...current, displayMessage('user', submittedMessage)].slice(-8));
    setRunState('checking_input');
    setResponse(null);
    setError('');

    const timers = [
      setTimeout(() => setRunState('generating'), 650),
      setTimeout(() => setRunState('checking_output'), 1_250),
    ];
    try {
      const result = await fetch(endpoint, {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ locale, sessionId, message: submittedMessage, history }),
      });
      if (!result.ok) {
        const payload = await result.json().catch(() => null);
        const publicMessage =
          payload !== null &&
          typeof payload === 'object' &&
          'error' in payload &&
          typeof payload.error === 'string'
            ? payload.error
            : copy.workflowFailed;
        const localizedMessage =
          locale === 'vi'
            ? result.status === 429
              ? copy.dailyLimit
              : copy.workflowFailed
            : publicMessage;
        throw new Error(localizedMessage);
      }

      const body = sanitizeContextualDemoResponse(await result.json());
      if (!body.modelCalled) {
        for (const timer of timers) clearTimeout(timer);
        setRunState('checking_input');
      }
      setResponse(body);
      if (body.policies.length > 0) {
        setPolicies(body.policies);
        setInventoryState('ready');
      }
      setMessages((current) => [...current, displayMessage('assistant', body.reply)].slice(-8));
      setRunState('success');
      trackMarketingEvent('contextual_demo_decision_shown', {
        page: pagePath,
        location: 'contextual_policy_monitor',
        scenario: selectedPreset ?? 'custom',
        decision: strongestEffect(body),
        outcome: 'success',
      });
    } catch (requestError) {
      setError(
        requestError instanceof Error
          ? requestError.message
          : copy.workflowFailed,
      );
      setRunState('error');
    } finally {
      for (const timer of timers) clearTimeout(timer);
    }
  }

  function choosePreset(path: CompanyDemoViewModel['paths'][number]): void {
    setMessage(path.proposal);
    setSelectedPreset(path.effect);
  }

  return (
    <div className={styles['companyDemo']} style={brandStyle}>
      <div className={`${styles['shell']} ${styles['healthcareShell']}`}>
        <section className={styles['chatPanel']} aria-labelledby="contextual-chat-title">
          <header className={styles['panelHeader']}>
            <div>
              <p>{profile.user_profile}</p>
              <h2 id="contextual-chat-title">{profile.workflow}</h2>
            </div>
            <span className={styles['companyName']}>{profile.company_name}</span>
          </header>

          <div className={styles['chatBody']} aria-live="polite">
            <div className={styles['syntheticBanner']} role="note">
              {copy.syntheticBanner}
            </div>
            {messages.map((entry) => (
              <div
                key={entry.id}
                className={
                  entry.role === 'assistant'
                    ? styles['assistantMessage']
                    : styles['customerMessage']
                }
              >
                <span>
                  {entry.role === 'assistant' ? copy.contextualAgent : copy.visitor}
                </span>
                <p>{entry.content}</p>
              </div>
            ))}
            {isRunning(runState) ? (
              <div className={styles['agentWorking']} role="status">
                <span className={styles['spinner']} aria-hidden="true" />
                {progressMessage(runState, locale)}
              </div>
            ) : null}
            {error !== '' ? (
              <div className={styles['errorMessage']} role="alert">
                <strong>{copy.replyStopped}</strong>
                <p>{error}</p>
              </div>
            ) : null}
          </div>

          <form className={styles['composer']} onSubmit={runDemo}>
            <div className={styles['exampleRow']} aria-label={copy.examplesLabel}>
              {profile.paths.map((path) => (
                <button
                  key={path.effect}
                  type="button"
                  className={selectedPreset === path.effect ? styles['selectedPreset'] : undefined}
                  onClick={() => choosePreset(path)}
                  disabled={isRunning(runState)}
                >
                  {path.label}
                </button>
              ))}
            </div>
            <label htmlFor="contextual-prompt">{copy.messageLabel}</label>
            <textarea
              id="contextual-prompt"
              value={message}
              onChange={(event) => {
                setMessage(event.target.value);
                setSelectedPreset(undefined);
              }}
              maxLength={500}
              rows={2}
            />
            <button className={styles['runButton']} type="submit" disabled={isRunning(runState)}>
              {isRunning(runState) ? copy.runningWorkflow : copy.send}
              <span aria-hidden="true">→</span>
            </button>
          </form>
        </section>

        <section className={styles['controlPanel']} aria-labelledby="contextual-monitor-title">
          <header className={styles['panelHeader']}>
            <div>
              <p>{copy.monitorKicker}</p>
              <h2 id="contextual-monitor-title">{copy.monitorTitle}</h2>
            </div>
            <DecisionBadge response={response} runState={runState} locale={locale} />
          </header>

          <div className={styles['healthcareMonitor']}>
            <section aria-labelledby="contextual-checks-title">
              <div className={styles['monitorSectionHeading']}>
                <h3 id="contextual-checks-title">{copy.protectedConversation}</h3>
                <span>{monitorSummary(runState, response, locale)}</span>
              </div>
              <div className={styles['checkTimeline']}>
                <CheckStep
                  number="01"
                  title={copy.inputBoundary}
                  detail={copy.inputDetail}
                  check={response?.checks[0]}
                  pending={runState === 'checking_input'}
                  locale={locale}
                />
                <CheckStep
                  number="02"
                  title="OpenAI Responses"
                  detail={
                    response?.modelCalled === false
                      ? copy.modelSkipped
                      : copy.modelDetail
                  }
                  pending={runState === 'generating'}
                  skipped={response?.modelCalled === false}
                  completed={response?.modelCalled === true}
                  locale={locale}
                />
                <CheckStep
                  number="03"
                  title={copy.outputBoundary}
                  detail={copy.outputDetail}
                  check={response?.checks[1]}
                  pending={runState === 'checking_output'}
                  locale={locale}
                />
              </div>
            </section>

            <section aria-labelledby="contextual-policies-title">
              <div className={styles['monitorSectionHeading']}>
                <h3 id="contextual-policies-title">{copy.policiesChecked}</h3>
                <span>{policySummary(inventoryState, policies, locale)}</span>
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
              <div className={styles['policyList']}>
                {policies.map((policy) => {
                  const matched = matchedPolicyIds.has(policy.id);
                  const scanning = isPolicyScanning(policy, runState);
                  return (
                    <article
                      key={policy.id}
                      className={`${styles['policyCard']} ${matched ? styles['matchedPolicy'] : ''} ${
                        scanning ? styles['scanningPolicy'] : ''
                      }`}
                    >
                      <span className={styles['policyDot']} aria-hidden="true" />
                      <div>
                        <strong>{localizedPolicyDescription(policy, locale)}</strong>
                        <code>{policy.id}</code>
                        <span className={styles['policyStatus']}>
                          {scanning
                            ? copy.checkingNow
                            : matched
                              ? copy.matched
                              : copy.ready}
                        </span>
                      </div>
                      <div className={styles['policyMeta']}>
                        <span>{copy.phase[policy.phase]}</span>
                        <span>{copy.severity[policy.severity]}</span>
                        <span>{copy.action[policy.action ?? 'check'] ?? policy.action ?? copy.action.check}</span>
                      </div>
                    </article>
                  );
                })}
              </div>
            </section>

            <div className={styles['companyPolicyRecord']}>
              <strong>{copy.decisionRecord}</strong>
              <p>{profile.record_shown}</p>
            </div>
          </div>

          <footer className={styles['conceptFooter']}>
            <p>
              {locale === 'vi'
                ? CONTEXTUAL_PAGE_COPY.vi.disclaimer(profile.company_name)
                : profile.disclaimer}
            </p>
          </footer>
        </section>
      </div>
    </div>
  );
}

function CheckStep({
  number,
  title,
  detail,
  check,
  pending = false,
  skipped = false,
  completed = false,
  locale,
}: {
  number: string;
  title: string;
  detail: string;
  check?: ContextualDemoResponse['checks'][number];
  pending?: boolean;
  skipped?: boolean;
  completed?: boolean;
  locale: HealthcareDemoLocale;
}) {
  const copy = CONTEXTUAL_UI_COPY[locale];
  const state = pending
    ? 'running'
    : skipped || check?.status === 'skipped'
      ? 'skipped'
      : check?.status === 'unavailable'
        ? 'unavailable'
        : completed
          ? 'permit'
          : check?.status === 'checked'
            ? check.effect ?? 'permit'
          : '';
  return (
    <article className={`${styles['checkStep']} ${state === '' ? '' : styles[state]}`}>
      <span>{number}</span>
      <div>
        <div className={styles['checkStepTitle']}>
          <h4>{title}</h4>
          <strong>
            {pending
              ? copy.checking
              : check?.status === 'unavailable'
                ? copy.unavailable
                : skipped || check?.status === 'skipped'
                  ? copy.skipped
                  : completed
                    ? copy.called
                  : check?.effect === undefined
                    ? copy.ready
                    : copy.effect[check.effect]}
          </strong>
        </div>
        <p>{localizedCheckReason(check, detail, pending, locale)}</p>
        {check?.traceId !== undefined ? <small>{copy.trace} {check.traceId}</small> : null}
      </div>
    </article>
  );
}

function DecisionBadge({
  response,
  runState,
  locale,
}: {
  response: ContextualDemoResponse | null;
  runState: RunState;
  locale: HealthcareDemoLocale;
}) {
  const copy = CONTEXTUAL_UI_COPY[locale];
  const effect = strongestEffect(response);
  const label = isRunning(runState)
    ? copy.checking
    : runState === 'error'
      ? copy.unavailable
      : response?.checks.some((check) => check.status === 'unavailable')
        ? copy.unavailable
      : response === null
        ? copy.ready
        : effect === 'defer' || effect === 'require_approval'
          ? copy.humanReview
          : copy.effect[effect];
  const badgeState =
    runState === 'error' || response?.checks.some((check) => check.status === 'unavailable')
      ? 'unavailable'
      : effect;
  return (
    <span className={`${styles['decisionBadge']} ${styles[badgeState] ?? ''}`}>{label}</span>
  );
}

function strongestEffect(response: ContextualDemoResponse | null): ContextualEffect {
  const rank = { permit: 0, transform: 1, require_approval: 2, defer: 3, deny: 4 } as const;
  return (
    response?.checks.reduce<keyof typeof rank>((strongest, check) => {
      const effect = check.effect ?? 'permit';
      return rank[effect] > rank[strongest] ? effect : strongest;
    }, 'permit') ?? 'permit'
  );
}

function displayMessage(role: DisplayMessage['role'], content: string): DisplayMessage {
  return { id: crypto.randomUUID(), role, content };
}

function isRunning(runState: RunState): boolean {
  return (
    runState === 'checking_input' || runState === 'generating' || runState === 'checking_output'
  );
}

function progressMessage(runState: RunState, locale: HealthcareDemoLocale): string {
  const copy = CONTEXTUAL_UI_COPY[locale];
  if (runState === 'checking_input') return copy.progressInput;
  if (runState === 'generating') return copy.progressModel;
  return copy.progressOutput;
}

function monitorSummary(
  runState: RunState,
  response: ContextualDemoResponse | null,
  locale: HealthcareDemoLocale,
): string {
  const copy = CONTEXTUAL_UI_COPY[locale];
  if (isRunning(runState)) return progressMessage(runState, locale);
  if (runState === 'error') return copy.failedClosed;
  if (response === null) return copy.waitingForMessage;
  return response.modelCalled ? copy.inputAndOutputChecked : copy.stoppedBeforeModel;
}

function policySummary(
  inventoryState: InventoryState,
  policies: ContextualPolicy[],
  locale: HealthcareDemoLocale,
): string {
  const copy = CONTEXTUAL_UI_COPY[locale];
  if (inventoryState === 'loading') return copy.loading;
  if (inventoryState === 'error') return copy.unavailable;
  return copy.policiesFromRust(policies.length);
}

function localizedCheckReason(
  check: ContextualDemoResponse['checks'][number] | undefined,
  detail: string,
  pending: boolean,
  locale: HealthcareDemoLocale,
): string {
  const copy = CONTEXTUAL_UI_COPY[locale];
  if (pending || check === undefined) return detail;
  if (locale === 'en') return check.reason ?? detail;
  if (check.status === 'unavailable') return copy.workflowFailed;
  if (check.status === 'skipped') return copy.stoppedBeforeModel;
  const policyId = check.findings.find((finding) => finding.policyId !== undefined)?.policyId;
  if (policyId !== undefined) return copy.matchedPolicy(policyId);
  if (check.effect === 'deny') return copy.policyBlocked;
  if (check.effect === 'transform') return copy.policyTransformed;
  if (check.effect === 'require_approval') return copy.policyNeedsReview;
  if (check.effect === 'defer') return copy.policyDeferred;
  return copy.noViolation;
}

function localizedPolicyDescription(
  policy: ContextualPolicy,
  locale: HealthcareDemoLocale,
): string {
  return CONTEXTUAL_UI_COPY[locale].policyDescriptions[policy.id] ?? policy.description ?? policy.id;
}

function isPolicyScanning(policy: ContextualPolicy, runState: RunState): boolean {
  return (
    (policy.phase === 'input' && runState === 'checking_input') ||
    (policy.phase === 'output' && runState === 'checking_output')
  );
}
