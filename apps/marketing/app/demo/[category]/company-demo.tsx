'use client';

import { useEffect, useMemo, useState, type CSSProperties, type FormEvent } from 'react';

import { trackMarketingEvent } from '@/lib/gtm';
import type { CompanyDemoViewModel } from '../company-profile';
import {
  sanitizeContextualDemoResponse,
  sanitizeContextualPolicyInventory,
  type ContextualDemoRequest,
  type ContextualDemoResponse,
  type ContextualPolicy,
} from '../contextual-contract';
import styles from '../demo.module.css';

type CompanyDemoProps = {
  profile: CompanyDemoViewModel;
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

interface DisplayMessage {
  id: string;
  role: 'user' | 'assistant';
  content: string;
}

export function CompanyDemo({ profile }: CompanyDemoProps) {
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
      content: `I’m a synthetic assistant for ${profile.workflow}. Ask a read-only question, request a shared change, or test the authorization boundary.`,
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
        if (!result.ok) throw new Error('Policy inventory request failed');
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
  }, [endpoint]);

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
      page: `/demo/${profile.slug}`,
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
        body: JSON.stringify({ sessionId, message: submittedMessage, history }),
      });
      if (!result.ok) {
        const payload = await result.json().catch(() => null);
        const publicMessage =
          payload !== null &&
          typeof payload === 'object' &&
          'error' in payload &&
          typeof payload.error === 'string'
            ? payload.error
            : 'The protected contextual workflow failed safely.';
        throw new Error(publicMessage);
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
        page: `/demo/${profile.slug}`,
        location: 'contextual_policy_monitor',
        scenario: selectedPreset ?? 'custom',
        decision: strongestEffect(body),
        outcome: 'success',
      });
    } catch (requestError) {
      setError(
        requestError instanceof Error
          ? requestError.message
          : 'The protected contextual workflow failed safely.',
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
              Synthetic concept only. Do not enter credentials, secrets, or private company data.
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
                <span>{entry.role === 'assistant' ? 'Contextual agent' : 'Visitor'}</span>
                <p>{entry.content}</p>
              </div>
            ))}
            {isRunning(runState) ? (
              <div className={styles['agentWorking']} role="status">
                <span className={styles['spinner']} aria-hidden="true" />
                {progressMessage(runState)}
              </div>
            ) : null}
            {error !== '' ? (
              <div className={styles['errorMessage']} role="alert">
                <strong>Reply stopped safely</strong>
                <p>{error}</p>
              </div>
            ) : null}
          </div>

          <form className={styles['composer']} onSubmit={runDemo}>
            <div className={styles['exampleRow']} aria-label="Example contextual requests">
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
            <label htmlFor="contextual-prompt">Message</label>
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
              {isRunning(runState) ? 'Running protected workflow' : 'Send through TrustLoopGuard'}
              <span aria-hidden="true">→</span>
            </button>
          </form>
        </section>

        <section className={styles['controlPanel']} aria-labelledby="contextual-monitor-title">
          <header className={styles['panelHeader']}>
            <div>
              <p>Shared demo workspace</p>
              <h2 id="contextual-monitor-title">TrustLoopGuard policy monitor</h2>
            </div>
            <DecisionBadge response={response} runState={runState} />
          </header>

          <div className={styles['healthcareMonitor']}>
            <section aria-labelledby="contextual-checks-title">
              <div className={styles['monitorSectionHeading']}>
                <h3 id="contextual-checks-title">Protected conversation</h3>
                <span>{monitorSummary(runState, response)}</span>
              </div>
              <div className={styles['checkTimeline']}>
                <CheckStep
                  number="01"
                  title="Input boundary"
                  detail="Checks the visitor message before OpenAI is called."
                  check={response?.checks[0]}
                  pending={runState === 'checking_input'}
                />
                <CheckStep
                  number="02"
                  title="OpenAI response"
                  detail={
                    response?.modelCalled === false
                      ? 'Skipped because the input decision stopped the request.'
                      : 'Uses bounded server-side workflow context and untrusted chat history.'
                  }
                  pending={runState === 'generating'}
                  skipped={response?.modelCalled === false}
                  completed={response?.modelCalled === true}
                />
                <CheckStep
                  number="03"
                  title="Output boundary"
                  detail="Checks the drafted response before it reaches the visitor."
                  check={response?.checks[1]}
                  pending={runState === 'checking_output'}
                />
              </div>
            </section>

            <section aria-labelledby="contextual-policies-title">
              <div className={styles['monitorSectionHeading']}>
                <h3 id="contextual-policies-title">Policies checked</h3>
                <span>{policySummary(inventoryState, policies)}</span>
              </div>
              {inventoryState === 'loading' ? (
                <p className={styles['inventoryNotice']} role="status">
                  Loading the Rust policy registry…
                </p>
              ) : null}
              {inventoryState === 'error' ? (
                <p className={styles['inventoryNotice']} role="status">
                  Policy inventory unavailable. Chat checks still fail closed.
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
                        <strong>{policy.description ?? policy.id}</strong>
                        <code>{policy.id}</code>
                        <span className={styles['policyStatus']}>
                          {scanning ? 'Checking now' : matched ? 'Matched' : 'Ready'}
                        </span>
                      </div>
                      <div className={styles['policyMeta']}>
                        <span>{policy.phase}</span>
                        <span>{policy.severity}</span>
                        <span>{policy.action ?? 'check'}</span>
                      </div>
                    </article>
                  );
                })}
              </div>
            </section>

            <div className={styles['companyPolicyRecord']}>
              <strong>Decision record</strong>
              <p>{profile.record_shown}</p>
            </div>
          </div>

          <footer className={styles['conceptFooter']}>
            <p>{profile.disclaimer}</p>
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
}: {
  number: string;
  title: string;
  detail: string;
  check?: ContextualDemoResponse['checks'][number];
  pending?: boolean;
  skipped?: boolean;
  completed?: boolean;
}) {
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
              ? 'Checking'
              : check?.status === 'unavailable'
                ? 'Unavailable'
                : skipped || check?.status === 'skipped'
                  ? 'Skipped'
                  : completed
                    ? 'Called'
                  : check?.effect ?? 'Ready'}
          </strong>
        </div>
        <p>{check?.reason ?? detail}</p>
        {check?.traceId !== undefined ? <small>Trace {check.traceId}</small> : null}
      </div>
    </article>
  );
}

function DecisionBadge({
  response,
  runState,
}: {
  response: ContextualDemoResponse | null;
  runState: RunState;
}) {
  const effect = strongestEffect(response);
  const label = isRunning(runState)
    ? 'Checking'
    : runState === 'error'
      ? 'Unavailable'
      : response?.checks.some((check) => check.status === 'unavailable')
        ? 'Unavailable'
      : response === null
        ? 'Ready'
        : effect === 'defer' || effect === 'require_approval'
          ? 'Human review'
          : effect.replace('_', ' ');
  const badgeState =
    runState === 'error' || response?.checks.some((check) => check.status === 'unavailable')
      ? 'unavailable'
      : effect;
  return (
    <span className={`${styles['decisionBadge']} ${styles[badgeState] ?? ''}`}>{label}</span>
  );
}

function strongestEffect(response: ContextualDemoResponse | null): string {
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

function progressMessage(runState: RunState): string {
  if (runState === 'checking_input') return 'TrustLoopGuard is checking the visitor message…';
  if (runState === 'generating') return 'OpenAI is drafting with bounded workflow context…';
  return 'TrustLoopGuard is checking the drafted reply…';
}

function monitorSummary(
  runState: RunState,
  response: ContextualDemoResponse | null,
): string {
  if (isRunning(runState)) return progressMessage(runState);
  if (runState === 'error') return 'Failed closed';
  if (response === null) return 'Waiting for a message';
  return response.modelCalled ? 'Input and output checked' : 'Stopped before model';
}

function policySummary(inventoryState: InventoryState, policies: ContextualPolicy[]): string {
  if (inventoryState === 'loading') return 'Loading';
  if (inventoryState === 'error') return 'Unavailable';
  return `${policies.length} from Rust`;
}

function isPolicyScanning(policy: ContextualPolicy, runState: RunState): boolean {
  return (
    (policy.phase === 'input' && runState === 'checking_input') ||
    (policy.phase === 'output' && runState === 'checking_output')
  );
}
