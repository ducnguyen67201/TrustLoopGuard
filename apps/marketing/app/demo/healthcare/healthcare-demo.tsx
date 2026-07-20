'use client';

import { useEffect, useMemo, useState, type FormEvent } from 'react';

import { trackMarketingEvent } from '@/lib/gtm';
import { DemoMeetingPrompt, useDemoMeetingPrompt } from '../demo-meeting-prompt';
import styles from '../demo.module.css';
import {
  HEALTHCARE_UI_COPY,
  type HealthcareDemoLocale,
} from './content';
import {
  sanitizeHealthcareDemoResponse,
  sanitizeHealthcarePolicyInventory,
  type HealthcareCheck,
  type HealthcareDemoRequest,
  type HealthcareDemoResponse,
  type HealthcarePolicy,
} from './contract';

const MODEL_DRAFT_START_MS = 700;
const OUTPUT_POLICY_SCAN_START_MS = 1_300;
const INPUT_POLICY_SCAN_MIN_MS = 900;
const FULL_POLICY_SCAN_MIN_MS = 2_100;

type RunState =
  | 'idle'
  | 'checking_input'
  | 'generating'
  | 'checking_output'
  | 'success'
  | 'error';
type InventoryState = 'loading' | 'ready' | 'error';

interface DisplayMessage {
  id: string;
  role: 'user' | 'assistant';
  content: string;
}

type HealthcareDemoPresentation = {
  companyName: string;
  workflow: string;
};

export function HealthcareDemo({
  locale = 'en',
  presentation,
}: {
  locale?: HealthcareDemoLocale;
  presentation?: HealthcareDemoPresentation;
}) {
  const { isMeetingPromptOpen, recordCompletedInteraction, dismissMeetingPrompt } =
    useDemoMeetingPrompt();
  const copy = HEALTHCARE_UI_COPY[locale];
  const [sessionId] = useState(() => crypto.randomUUID());
  const [message, setMessage] = useState<string>(copy.presets[0].message);
  const [selectedPreset, setSelectedPreset] = useState<string>(copy.presets[0].id);
  const [messages, setMessages] = useState<DisplayMessage[]>([
    { id: 'greeting', role: 'assistant', content: copy.greeting },
  ]);
  const [runState, setRunState] = useState<RunState>('idle');
  const [response, setResponse] = useState<HealthcareDemoResponse | null>(null);
  const [policies, setPolicies] = useState<HealthcarePolicy[]>([]);
  const [inventoryState, setInventoryState] = useState<InventoryState>('loading');
  const [error, setError] = useState('');

  useEffect(() => {
    let active = true;
    async function loadPolicies(): Promise<void> {
      try {
        const result = await fetch('/api/demo/healthcare', { cache: 'no-store' });
        if (!result.ok) throw new Error(copy.inventoryRequestFailed);
        const inventory = sanitizeHealthcarePolicyInventory(await result.json());
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
  }, [copy.inventoryRequestFailed]);

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
    const runStartedAt = performance.now();

    const preset = copy.presets.find((candidate) => candidate.message === submittedMessage);
    const scenario = preset?.id ?? 'custom';
    const history: HealthcareDemoRequest['history'] = messages.slice(-8).map((entry) => ({
      role: entry.role,
      content: entry.content,
    }));

    trackMarketingEvent('healthcare_demo_started', {
      page: copy.pagePath,
      location: 'healthcare_composer',
      scenario,
    });

    setMessages((current) =>
      [...current, displayMessage('user', submittedMessage)].slice(-8),
    );
    setRunState('checking_input');
    setResponse(null);
    setError('');

    const timers: Array<ReturnType<typeof setTimeout>> = [];
    if (preset?.stopsAtInput !== true) {
      timers.push(setTimeout(() => setRunState('generating'), MODEL_DRAFT_START_MS));
      timers.push(setTimeout(() => setRunState('checking_output'), OUTPUT_POLICY_SCAN_START_MS));
    }

    try {
      const result = await fetch('/api/demo/healthcare', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ locale, sessionId, message: submittedMessage, history }),
      });
      if (!result.ok) {
        const errorPayload = await result.json().catch(() => null);
        const publicMessage =
          errorPayload !== null &&
          typeof errorPayload === 'object' &&
          'error' in errorPayload &&
          typeof errorPayload.error === 'string'
            ? locale === 'vi'
              ? result.status === 429
                ? copy.dailyLimit
                : copy.workflowFailed
              : errorPayload.error
            : copy.workflowFailed;
        throw new Error(publicMessage);
      }

      const body = sanitizeHealthcareDemoResponse(await result.json());
      if (!body.modelCalled) {
        for (const timer of timers) clearTimeout(timer);
        setRunState('checking_input');
      }
      await waitForMinimumDuration(
        runStartedAt,
        body.modelCalled ? FULL_POLICY_SCAN_MIN_MS : INPUT_POLICY_SCAN_MIN_MS,
      );
      setResponse(body);
      if (body.policies.length > 0) {
        setPolicies(body.policies);
        setInventoryState('ready');
      }
      setMessages((current) => [...current, displayMessage('assistant', body.reply)].slice(-8));
      setRunState('success');
      recordCompletedInteraction();
      trackMarketingEvent('healthcare_demo_decision_shown', {
        page: copy.pagePath,
        location: 'healthcare_policy_monitor',
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
      trackMarketingEvent('healthcare_demo_decision_shown', {
        page: copy.pagePath,
        location: 'healthcare_policy_monitor',
        scenario,
        decision: 'request_error',
        outcome: 'error',
      });
    } finally {
      for (const timer of timers) clearTimeout(timer);
    }
  }

  function choosePreset(preset: (typeof copy.presets)[number]): void {
    setMessage(preset.message);
    setSelectedPreset(preset.id);
  }

  return (
    <div className={`${styles['shell']} ${styles['healthcareShell']}`}>
      <section className={styles['chatPanel']} aria-labelledby="healthcare-chat-title">
        <div className={styles['panelHeader']}>
          <div>
            <p>
              {presentation ? copy.preparedFor(presentation.companyName) : copy.chatKicker}
            </p>
            <h2 id="healthcare-chat-title">
              {presentation?.workflow ?? copy.chatTitle}
            </h2>
          </div>
          <span className={styles['liveBadge']}>
            <i aria-hidden="true" /> {copy.protected}
          </span>
        </div>

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
              <span>{entry.role === 'assistant' ? copy.assistantName : copy.visitor}</span>
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
          <div className={styles['exampleRow']} aria-label={copy.scenariosLabel}>
            {copy.presets.map((preset) => (
              <button
                key={preset.id}
                type="button"
                className={selectedPreset === preset.id ? styles['selectedPreset'] : undefined}
                onClick={() => choosePreset(preset)}
                disabled={isRunning(runState)}
              >
                {preset.label}
              </button>
            ))}
          </div>
          <label htmlFor="healthcare-message">{copy.messageLabel}</label>
          <textarea
            id="healthcare-message"
            value={message}
            onChange={(event) => {
              setMessage(event.target.value);
              setSelectedPreset('custom');
            }}
            maxLength={500}
            rows={3}
          />
          <button className={styles['runButton']} type="submit" disabled={isRunning(runState)}>
            {isRunning(runState) ? copy.runningWorkflow : copy.send}
            <span aria-hidden="true">→</span>
          </button>
        </form>
      </section>

      <section className={styles['controlPanel']} aria-labelledby="healthcare-policy-title">
        <div className={styles['panelHeader']}>
          <div>
            <p>{copy.monitorKicker}</p>
            <h2 id="healthcare-policy-title">{copy.monitorTitle}</h2>
          </div>
          <EffectBadge effect={latestEffect(response)} locale={locale} />
        </div>

        <div className={styles['healthcareMonitor']}>
          <section aria-labelledby="turn-checks-title">
            <div className={styles['monitorSectionHeading']}>
              <h3 id="turn-checks-title">{copy.thisTurn}</h3>
              <span>{response === null ? copy.readyForMessage : copy.guardedResult}</span>
            </div>
            <div className={styles['checkTimeline']}>
              <CheckStep
                number="01"
                title={copy.inputBoundary}
                check={response?.checks[0]}
                pending={runState === 'checking_input'}
                locale={locale}
              />
              <ModelStep runState={runState} response={response} locale={locale} />
              <CheckStep
                number="03"
                title={copy.outputBoundary}
                check={response?.checks[1]}
                pending={runState === 'checking_output'}
                locale={locale}
              />
            </div>
          </section>

          <section aria-labelledby="policy-checks-title" aria-busy={isRunning(runState)}>
            <div className={styles['monitorSectionHeading']}>
              <h3 id="policy-checks-title">{copy.policiesChecked}</h3>
              <span aria-live="polite">
                {policyMonitorSummary(runState, policies, inventoryState, locale)}
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
            {inventoryState === 'ready' && policies.length === 0 ? (
              <p className={styles['inventoryNotice']}>{copy.noPolicies}</p>
            ) : null}
            <div className={styles['policyList']}>
              {policies.map((policy) => {
                const matched = matchedPolicyIds.has(policy.id);
                const scanning = isPolicyScanning(policy, runState);
                const phaseCheck = policyPhaseCheck(policy, response);
                return (
                  <article
                    key={policy.id}
                    className={`${styles['policyCard']} ${
                      matched ? styles['matchedPolicy'] : ''
                    } ${scanning ? styles['scanningPolicy'] : ''} ${
                      phaseCheck?.status === 'checked' ? styles['checkedPolicy'] : ''
                    } ${phaseCheck?.status === 'skipped' ? styles['skippedPolicy'] : ''}`}
                  >
                    <span className={styles['policyDot']} aria-hidden="true" />
                    <div>
                      <strong>{localizedPolicyDescription(policy, locale)}</strong>
                      <code>{policy.id}</code>
                      <span className={styles['policyStatus']}>
                        {policyStatusLabel(scanning, matched, phaseCheck, locale)}
                      </span>
                    </div>
                    <div className={styles['policyMeta']}>
                      {policy.phase !== undefined ? <span>{copy.phase[policy.phase]}</span> : null}
                      <span>{copy.severity[policy.severity]}</span>
                      <span>{copy.action[policy.action ?? 'check'] ?? policy.action ?? 'check'}</span>
                    </div>
                  </article>
                );
              })}
            </div>
          </section>
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

function CheckStep({
  number,
  title,
  check,
  pending,
  locale,
}: {
  number: string;
  title: string;
  check?: HealthcareCheck;
  pending: boolean;
  locale: HealthcareDemoLocale;
}) {
  const copy = HEALTHCARE_UI_COPY[locale];
  const state = pending ? 'running' : (check?.status ?? 'idle');
  const label = pending
    ? copy.checking
    : check?.effect === undefined
      ? checkStatusLabel(check?.status, locale)
      : effectLabel(check.effect, locale);
  return (
    <article
      className={`${styles['checkStep']} ${styles[state]} ${
        check?.effect === undefined ? '' : styles[check.effect]
      }`}
    >
      <span>{number}</span>
      <div>
        <div className={styles['checkStepTitle']}>
          <h4>{title}</h4>
          <strong>{label}</strong>
        </div>
        <p>{localizedCheckReason(check, pending, locale)}</p>
        {check?.traceId !== undefined ? (
          <small>
            {locale === 'vi' ? 'truy vết' : 'trace'} {check.traceId} · {check.latencyMs ?? 0}ms
          </small>
        ) : null}
      </div>
    </article>
  );
}

function ModelStep({
  runState,
  response,
  locale,
}: {
  runState: RunState;
  response: HealthcareDemoResponse | null;
  locale: HealthcareDemoLocale;
}) {
  const copy = HEALTHCARE_UI_COPY[locale];
  const running = runState === 'generating';
  const label = running
    ? copy.drafting
    : response === null
      ? copy.ready
      : response.modelCalled
        ? copy.calledOnce
        : copy.skipped;
  return (
    <article
      className={`${styles['checkStep']} ${styles[running ? 'running' : response === null ? 'idle' : 'checked']}`}
    >
      <span>02</span>
      <div>
        <div className={styles['checkStepTitle']}>
          <h4>OpenAI Responses</h4>
          <strong>{label}</strong>
        </div>
        <p>
          {response?.modelCalled === false
            ? copy.modelStopped
            : copy.modelDescription}
        </p>
      </div>
    </article>
  );
}

function EffectBadge({
  effect,
  locale,
}: {
  effect: HealthcareCheck['effect'] | 'ready';
  locale: HealthcareDemoLocale;
}) {
  return (
    <span className={`${styles['decisionBadge']} ${styles[effect ?? 'ready']}`}>
      {effect === 'ready' || effect === undefined
        ? HEALTHCARE_UI_COPY[locale].ready
        : effectLabel(effect, locale)}
    </span>
  );
}

function latestEffect(
  response: HealthcareDemoResponse | null,
): HealthcareCheck['effect'] | 'ready' {
  return response?.checks[1].effect ?? response?.checks[0].effect ?? 'ready';
}

function analyticsDecision(response: HealthcareDemoResponse): string {
  const check = response.checks[1].effect === undefined ? response.checks[0] : response.checks[1];
  return check.effect ?? check.status;
}

function effectLabel(
  effect: NonNullable<HealthcareCheck['effect']>,
  locale: HealthcareDemoLocale,
): string {
  return HEALTHCARE_UI_COPY[locale].effect[effect];
}

function progressMessage(runState: RunState, locale: HealthcareDemoLocale): string {
  const copy = HEALTHCARE_UI_COPY[locale];
  if (runState === 'checking_input') return copy.progressInput;
  if (runState === 'generating') return copy.progressModel;
  return copy.progressOutput;
}

function checkStatusLabel(
  status: HealthcareCheck['status'] | undefined,
  locale: HealthcareDemoLocale,
): string {
  const copy = HEALTHCARE_UI_COPY[locale];
  if (status === 'checked') return copy.checkedThisTurn;
  if (status === 'skipped') return copy.skipped;
  if (status === 'unavailable') return copy.unavailable;
  return copy.ready;
}

function localizedCheckReason(
  check: HealthcareCheck | undefined,
  pending: boolean,
  locale: HealthcareDemoLocale,
): string {
  const copy = HEALTHCARE_UI_COPY[locale];
  if (pending) return copy.evaluatingPolicies;
  if (check?.status === 'skipped') return copy.skippedEarlier;
  if (check?.status === 'unavailable') return copy.guardUnavailable;
  if (check?.status !== 'checked') return copy.waitingMessage;
  if (locale === 'en') return check.reason ?? copy.noViolation;
  const policyId = check.findings.find((finding) => finding.policyId !== undefined)?.policyId;
  if (policyId !== undefined) return copy.matchedPolicy(policyId);
  if (check.effect === 'transform') return copy.policyTransformed;
  if (check.effect === 'deny') return copy.policyBlocked;
  if (check.effect === 'require_approval') return copy.policyNeedsReview;
  if (check.effect === 'defer') return copy.policyDeferred;
  return copy.noViolation;
}

function displayMessage(role: DisplayMessage['role'], content: string): DisplayMessage {
  return { id: crypto.randomUUID(), role, content };
}

function policyMonitorSummary(
  runState: RunState,
  policies: HealthcarePolicy[],
  inventoryState: InventoryState,
  locale: HealthcareDemoLocale,
): string {
  const copy = HEALTHCARE_UI_COPY[locale];
  if (runState === 'checking_input') {
    return copy.inputChecksRunning(policyPhaseCount(policies, 'input'));
  }
  if (runState === 'generating') return copy.inputChecksPassed;
  if (runState === 'checking_output') {
    return copy.outputChecksRunning(policyPhaseCount(policies, 'output'));
  }
  if (inventoryState === 'loading') return copy.loading;
  if (inventoryState === 'error') return copy.unavailable;
  return copy.activePolicies(policies.length);
}

function policyPhaseCount(
  policies: HealthcarePolicy[],
  phase: NonNullable<HealthcarePolicy['phase']>,
): number {
  return policies.filter((policy) => policy.phase === phase).length;
}

function isPolicyScanning(policy: HealthcarePolicy, runState: RunState): boolean {
  return (
    (policy.phase === 'input' && runState === 'checking_input') ||
    (policy.phase === 'output' && runState === 'checking_output')
  );
}

function policyPhaseCheck(
  policy: HealthcarePolicy,
  response: HealthcareDemoResponse | null,
): HealthcareCheck | undefined {
  if (policy.phase === 'input') return response?.checks[0];
  if (policy.phase === 'output') return response?.checks[1];
  return undefined;
}

function policyStatusLabel(
  scanning: boolean,
  matched: boolean,
  phaseCheck: HealthcareCheck | undefined,
  locale: HealthcareDemoLocale,
): string {
  const copy = HEALTHCARE_UI_COPY[locale];
  if (scanning) return copy.checkingNow;
  if (matched) return copy.matchedThisTurn;
  if (phaseCheck?.status === 'checked') return copy.checkedThisTurn;
  if (phaseCheck?.status === 'skipped') return copy.skippedThisTurn;
  if (phaseCheck?.status === 'unavailable') return copy.checkUnavailable;
  return copy.activeInRust;
}

function localizedPolicyDescription(
  policy: HealthcarePolicy,
  locale: HealthcareDemoLocale,
): string {
  return HEALTHCARE_UI_COPY[locale].policyDescriptions[policy.id] ?? policy.description ?? policy.id;
}

async function waitForMinimumDuration(startedAt: number, minimumMs: number): Promise<void> {
  const remainingMs = minimumMs - (performance.now() - startedAt);
  if (remainingMs <= 0) return;
  await new Promise<void>((resolve) => setTimeout(resolve, remainingMs));
}

function isRunning(runState: RunState): boolean {
  return (
    runState === 'checking_input' ||
    runState === 'generating' ||
    runState === 'checking_output'
  );
}
