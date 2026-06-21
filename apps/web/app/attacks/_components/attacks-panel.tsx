'use client';

import {
  ChevronDown,
  Clock,
  Crosshair,
  Radar,
  ShieldAlert,
  ShieldCheck,
  Sparkles,
  Swords,
  Target,
  X,
} from 'lucide-react';
import { useCallback, useEffect, useRef, useState, type ReactNode } from 'react';

import {
  REDTEAM_ATTACK_SURFACES,
  REDTEAM_JOB_PROFILES,
  REDTEAM_RUN_MODES,
  isTerminalStatus,
  landedPercent,
  redteam,
  type DocumentTemplateInput,
  type JobStatus,
  type RedteamAttackSurface,
  type RedteamJobProfile,
  type RedteamJobResult,
  type RedteamJobSummary,
  type RedteamRunMode,
} from '@/lib/redteam-jobs';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardHeader, CardTitle } from '@/components/ui/card';
import { InfoHint } from '@/components/ui/info-hint';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { PageHeader } from '@/components/ui/page-header';
import { cn } from '@/lib/utils';
import { listAgents, type AgentSummary } from '@/lib/agents';
import {
  deletePlan,
  generateStaticPolicies,
  listPlans,
  planAttackVectors,
  type RedteamPlan,
} from '@/lib/redteam-plan';

import { HardenJobCard } from './harden-job-card';
import { PlanStep, PlanVectors } from './plan-card';
import { ReportShareCard } from './report-share-card';

const POLL_INTERVAL_MS = 1200;
const DEFAULT_TARGET = 'http://127.0.0.1:9102';
const HISTORY_LIMIT = 10;
const MAX_DOCUMENT_TEMPLATE_BYTES = 10 * 1024 * 1024;

const PROFILE_COPY: Record<RedteamJobProfile, string> = {
  fast: 'Fast — a quick spot check with a handful of attacks. About a minute.',
  full: 'Full — one of every kind of attack. A few minutes, more thorough.',
  max: 'Max — every attack, tried many ways. The slowest and most thorough.',
};

const MODE_COPY: Record<RedteamRunMode, string> = {
  one_off: 'One-off — a fresh test that starts from scratch every time.',
  learning: 'Learning — keeps notes between tests to probe a little smarter.',
};

const SURFACE_COPY: Record<RedteamAttackSurface, string> = {
  chat: 'Chat — sends tricky messages, like a user typing to your agent.',
  document_workflow: 'Document — hides the attack inside an uploaded PDF form.',
};

const delay = (ms: number) => new Promise<void>((resolve) => setTimeout(resolve, ms));

function messageOf(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

export function AttacksPanel() {
  const [targetUrl, setTargetUrl] = useState(DEFAULT_TARGET);
  const [profile, setProfile] = useState<RedteamJobProfile>('fast');
  const [mode, setMode] = useState<RedteamRunMode>('one_off');
  const [attackSurface, setAttackSurface] = useState<RedteamAttackSurface>('chat');
  const [job, setJob] = useState<RedteamJobSummary | null>(null);
  const [results, setResults] = useState<RedteamJobResult[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [expanded, setExpanded] = useState<number | null>(null);
  const [dispatching, setDispatching] = useState(false);
  const [preparingTemplate, setPreparingTemplate] = useState(false);
  const [history, setHistory] = useState<RedteamJobSummary[]>([]);
  const [documentTemplateFile, setDocumentTemplateFile] = useState<File | null>(null);
  const [documentTemplateFlatten, setDocumentTemplateFlatten] = useState(false);

  // Hardening loop step 1: pick an imported agent, plan tailored vectors.
  const [agents, setAgents] = useState<AgentSummary[]>([]);
  const [selectedAgentId, setSelectedAgentId] = useState<string | null>(null);
  const [planName, setPlanName] = useState('');
  const [plan, setPlan] = useState<RedteamPlan | null>(null);
  const [savedPlans, setSavedPlans] = useState<RedteamPlan[]>([]);
  const [planning, setPlanning] = useState(false);
  const [planError, setPlanError] = useState<string | null>(null);
  const [staticBusy, setStaticBusy] = useState(false);
  const [staticCount, setStaticCount] = useState<number | null>(null);

  // The id currently being polled. Setting it to null breaks the poll loop —
  // used by cancel, unmount, and starting/loading a different job.
  const activeJobRef = useRef<string | null>(null);
  // The agent whose saved plans we last requested. Guards against a slow
  // listPlans for agent A resolving after the user has switched to agent B and
  // overwriting B's plans with A's.
  const plansAgentRef = useRef<string | null>(null);
  useEffect(
    () => () => {
      activeJobRef.current = null;
    },
    [],
  );

  // The detail pane. On the stacked mobile layout it sits below the selector, so
  // selecting a job scrolls it into view; on desktop both panes are already visible.
  const detailRef = useRef<HTMLDivElement>(null);
  const revealDetailOnMobile = useCallback(() => {
    // Guard for non-DOM/test environments (jsdom has no matchMedia/scrollIntoView).
    if (typeof window === 'undefined' || typeof window.matchMedia !== 'function') return;
    if (window.matchMedia('(min-width: 1024px)').matches) return;
    detailRef.current?.scrollIntoView?.({ behavior: 'smooth', block: 'start' });
  }, []);

  // A finished run's report (summary, evidence, harden card, error) must not
  // linger under a new configuration. Editing the target, switching agents, or
  // choosing/building a plan returns the right pane to the draft plan view.
  const clearStaleRun = useCallback(() => {
    if (job === null && error === null) return;
    activeJobRef.current = null;
    setJob(null);
    setResults([]);
    setError(null);
    setExpanded(null);
  }, [job, error]);

  const refreshHistory = useCallback(async () => {
    try {
      setHistory(await redteam.listJobs({ limit: HISTORY_LIMIT }));
    } catch {
      // History is best-effort; a failure here must not break the active run.
    }
  }, []);

  useEffect(() => {
    void refreshHistory();
  }, [refreshHistory]);

  useEffect(() => {
    void (async () => {
      try {
        setAgents(await listAgents());
      } catch {
        // Agent list is best-effort; planning is optional.
      }
    })();
  }, []);

  const onSelectAgent = useCallback(
    (id: string | null) => {
      clearStaleRun();
      setSelectedAgentId(id);
      setPlan(null);
      setPlanError(null);
      setStaticCount(null);
      setPlanName('');
      setSavedPlans([]);
      // Agent-first: derive the target from the agent's saved connection so the
      // user never re-types it. No agent (generic) → the loopback default.
      const agent = agents.find((a) => a.agentId === id);
      setTargetUrl(agent?.targetUrl ?? DEFAULT_TARGET);
      // ...and derive the surface too: a workflow agent is attacked via the
      // document-workflow surface (PDF upload to /arena/workflow), a chat agent
      // via /v1. Without this the runner pings /v1 on a workflow target → 404.
      setAttackSurface(agent?.hasWorkflow ? 'document_workflow' : 'chat');
      plansAgentRef.current = id;
      if (id === null) return;
      void (async () => {
        try {
          const plans = await listPlans(id);
          // Drop the result if the user has since switched agents.
          if (plansAgentRef.current === id) setSavedPlans(plans);
        } catch {
          // Saved-plan list is best-effort; planning still works without it.
        }
      })();
    },
    [agents, clearStaleRun],
  );

  const onPlan = useCallback(async () => {
    const agentId = selectedAgentId;
    if (agentId === null) return;
    setPlanning(true);
    setPlanError(null);
    setStaticCount(null);
    try {
      const saved = await planAttackVectors(agentId, planName);
      // Drop the result if the user switched agents mid-plan — otherwise this
      // plan (and the vectors it seeds a dispatch with) would bind to the wrong
      // agent. `plansAgentRef` tracks the live selection.
      if (plansAgentRef.current !== agentId) return;
      clearStaleRun();
      setPlan(saved);
      setSavedPlans((prev) => [saved, ...prev]);
      setPlanName('');
    } catch (err) {
      if (plansAgentRef.current !== agentId) return;
      setPlanError(messageOf(err));
    } finally {
      setPlanning(false);
    }
  }, [selectedAgentId, planName, clearStaleRun]);

  const onSelectPlan = useCallback((selected: RedteamPlan) => {
    clearStaleRun();
    setPlan(selected);
    setPlanError(null);
    setStaticCount(null);
  }, [clearStaleRun]);

  const onDeletePlan = useCallback(
    async (planId: string) => {
      try {
        await deletePlan(planId);
        setSavedPlans((prev) => prev.filter((p) => p.id !== planId));
        setPlan((current) => (current?.id === planId ? null : current));
      } catch (err) {
        setPlanError(messageOf(err));
      }
    },
    [],
  );

  const onGenerateStatic = useCallback(async () => {
    if (selectedAgentId === null) return;
    setStaticBusy(true);
    setPlanError(null);
    try {
      const result = await generateStaticPolicies(selectedAgentId);
      setStaticCount(result.generated.length);
    } catch (err) {
      setPlanError(messageOf(err));
    } finally {
      setStaticBusy(false);
    }
  }, [selectedAgentId]);

  const busy = dispatching || preparingTemplate || (job !== null && !isTerminalStatus(job.status));

  const poll = useCallback(
    async (id: string) => {
      while (activeJobRef.current === id) {
        let detail;
        try {
          detail = await redteam.getJob(id);
        } catch (err) {
          setError(messageOf(err));
          activeJobRef.current = null;
          return;
        }
        setJob(detail.job);
        setResults(detail.results);
        if (isTerminalStatus(detail.job.status)) {
          if (detail.job.status === 'error') {
            setError(detail.job.error ?? 'the attack job failed');
          }
          activeJobRef.current = null;
          void refreshHistory();
          return;
        }
        await delay(POLL_INTERVAL_MS);
      }
    },
    [refreshHistory],
  );

  const run = useCallback(async () => {
    const target = targetUrl.trim();
    if (target === '') {
      setError('Enter your agent URL first.');
      return;
    }
    // Stop any in-flight poll before re-dispatching so a stale getJob can't
    // write the previous job's state over the new one.
    activeJobRef.current = null;
    setDispatching(true);
    setPreparingTemplate(false);
    setError(null);
    setJob(null);
    setResults([]);
    setExpanded(null);

    let documentTemplate: DocumentTemplateInput | undefined;
    if (attackSurface === 'document_workflow' && documentTemplateFile !== null) {
      setPreparingTemplate(true);
      try {
        documentTemplate = await buildDocumentTemplate({
          file: documentTemplateFile,
          flatten: documentTemplateFlatten,
        });
      } catch (err) {
        setDispatching(false);
        setPreparingTemplate(false);
        setError(messageOf(err));
        return;
      }
      setPreparingTemplate(false);
    }

    let summary: RedteamJobSummary;
    try {
      // Associate the agent (so harden attaches its verified policies to it) and
      // seed the run with the planned vectors (so the attack is tailored).
      const dispatchInput = {
        targetUrl: target,
        profile,
        mode,
        attackSurface,
        ...(selectedAgentId !== null ? { agentId: selectedAgentId } : {}),
        ...(plan !== null && plan.vectors.length > 0 ? { attackVectors: plan.vectors } : {}),
      };
      summary = await redteam.dispatch(
        documentTemplate === undefined ? dispatchInput : { ...dispatchInput, documentTemplate },
      );
    } catch (err) {
      setDispatching(false);
      setError(messageOf(err));
      return;
    }
    setJob(summary);
    setDispatching(false);
    revealDetailOnMobile();
    activeJobRef.current = summary.id;
    await poll(summary.id);
  }, [
    attackSurface,
    documentTemplateFile,
    documentTemplateFlatten,
    mode,
    profile,
    targetUrl,
    selectedAgentId,
    plan,
    poll,
    revealDetailOnMobile,
  ]);

  const cancel = useCallback(async () => {
    if (job === null) return;
    activeJobRef.current = null;
    try {
      setJob(await redteam.cancel(job.id));
      void refreshHistory();
    } catch (err) {
      setError(messageOf(err));
    }
  }, [job, refreshHistory]);

  const loadFromHistory = useCallback(
    async (id: string) => {
      activeJobRef.current = null;
      setError(null);
      setExpanded(null);
      try {
        const detail = await redteam.getJob(id);
        setJob(detail.job);
        setResults(detail.results);
        revealDetailOnMobile();
        if (!isTerminalStatus(detail.job.status)) {
          activeJobRef.current = id;
          await poll(id);
        }
      } catch (err) {
        setError(messageOf(err));
      }
    },
    [poll, revealDetailOnMobile],
  );

  const hasDetail = job !== null || error !== null;

  return (
    <div className="grid w-full max-w-full min-w-0 gap-6 px-4 py-4 lg:px-6 lg:py-6">
      <PageHeader
        eyebrow="Red-team testing"
        title="Test your AI agent"
        help={<InfoHint term="redteam" />}
        description="Safely test your own AI agent by sending it tricky and abusive prompts, to see how well your guardrails hold up. Tests run in the background, so you can leave and come back — your results will be waiting."
      />

      {/* Master–detail: choose a target / past job on the left, read its results on the right. */}
      <div className="grid w-full max-w-full min-w-0 gap-6 lg:grid-cols-[minmax(0,360px)_minmax(0,1fr)] lg:items-start">
        <div className="grid w-full max-w-full min-w-0 gap-4">
          <AttackFlow
            agents={agents}
            selectedAgentId={selectedAgentId}
            onSelectAgent={onSelectAgent}
            connectedAgentName={
              agents.find((a) => a.agentId === selectedAgentId)?.displayName ?? null
            }
            targetUrl={targetUrl}
            onTargetChange={(value) => {
              setTargetUrl(value);
              clearStaleRun();
            }}
            planName={planName}
            onPlanNameChange={setPlanName}
            plan={plan}
            planning={planning}
            planError={planError}
            onPlan={() => void onPlan()}
            savedPlans={savedPlans}
            onSelectPlan={onSelectPlan}
            onDeletePlan={(planId) => void onDeletePlan(planId)}
            staticBusy={staticBusy}
            staticCount={staticCount}
            onGenerateStatic={() => void onGenerateStatic()}
            profile={profile}
            mode={mode}
            attackSurface={attackSurface}
            busy={busy}
            canCancel={busy && job !== null}
            onSelectProfile={(value) => {
              if (value === profile) return;
              clearStaleRun();
              setProfile(value);
            }}
            onSelectMode={(value) => {
              if (value === mode) return;
              clearStaleRun();
              setMode(value);
            }}
            onSelectAttackSurface={(value) => {
              if (value === attackSurface) return;
              clearStaleRun();
              setAttackSurface(value);
            }}
            documentTemplateFile={documentTemplateFile}
            documentTemplateFlatten={documentTemplateFlatten}
            onDocumentTemplateFileChange={(value) => {
              clearStaleRun();
              setDocumentTemplateFile(value);
            }}
            onDocumentTemplateFlattenChange={(value) => {
              clearStaleRun();
              setDocumentTemplateFlatten(value);
            }}
            onRun={() => void run()}
            onCancel={() => void cancel()}
          />

          {history.length > 0 ? (
            <JobHistory jobs={history} activeId={job?.id ?? null} onSelect={loadFromHistory} />
          ) : null}
        </div>

        <div ref={detailRef} className="grid w-full max-w-full min-w-0 content-start gap-6">
          {error ? (
            <p
              role="alert"
              className="flex items-start gap-2.5 rounded-lg border border-destructive/40 border-l-4 border-l-destructive bg-destructive/10 px-3.5 py-2.5 text-sm text-destructive"
            >
              <ShieldAlert className="mt-0.5 size-4 shrink-0" aria-hidden="true" />
              <span>{error}</span>
            </p>
          ) : null}

          {job ? <ResultSummary job={job} /> : null}

          {results.length > 0 ? (
            <ThreatResultBoard results={results} expanded={expanded} onToggle={setExpanded} />
          ) : job !== null && !isTerminalStatus(job.status) ? (
            <ScanningBoard target={job.target} />
          ) : null}

          {/* Before a run, the planned vectors fill the wide pane so they're
              readable in one view; a job's results take over once it starts. */}
          {plan !== null && job === null ? <PlanVectors plan={plan} /> : null}

          {job?.status === 'complete' ? (
            <HardenJobCard
              jobId={job?.id ?? null}
              results={results}
              busy={busy}
              onHardened={() => void run()}
            />
          ) : null}

          {job?.status === 'complete' ? <ReportShareCard job={job} /> : null}

          {hasDetail || (plan !== null && job === null) ? null : dispatching ? (
            <ScanningBoard target={targetUrl.trim() || 'target'} />
          ) : (
            <DetailEmptyState hasHistory={history.length > 0} />
          )}
        </div>
      </div>
    </div>
  );
}

interface AttackFlowProps {
  agents: readonly AgentSummary[];
  selectedAgentId: string | null;
  onSelectAgent: (id: string | null) => void;
  connectedAgentName: string | null;
  targetUrl: string;
  onTargetChange: (value: string) => void;
  planName: string;
  onPlanNameChange: (value: string) => void;
  plan: RedteamPlan | null;
  planning: boolean;
  planError: string | null;
  onPlan: () => void;
  savedPlans: readonly RedteamPlan[];
  onSelectPlan: (plan: RedteamPlan) => void;
  onDeletePlan: (planId: string) => void;
  staticBusy: boolean;
  staticCount: number | null;
  onGenerateStatic: () => void;
  profile: RedteamJobProfile;
  mode: RedteamRunMode;
  attackSurface: RedteamAttackSurface;
  busy: boolean;
  canCancel: boolean;
  onSelectProfile: (value: RedteamJobProfile) => void;
  onSelectMode: (value: RedteamRunMode) => void;
  onSelectAttackSurface: (value: RedteamAttackSurface) => void;
  documentTemplateFile: File | null;
  documentTemplateFlatten: boolean;
  onDocumentTemplateFileChange: (value: File | null) => void;
  onDocumentTemplateFlattenChange: (value: boolean) => void;
  onRun: () => void;
  onCancel: () => void;
}

/**
 * The whole left column is one card with three numbered steps —
 * 1 Agent → 2 Plan → 3 Attack — joined by a vertical rail so a first-timer reads
 * it as a single sequence ending in the one orange Attack CTA. Each step's
 * controls sit tightly under its label; nothing else competes for "primary".
 */
function AttackFlow({
  agents,
  selectedAgentId,
  onSelectAgent,
  connectedAgentName,
  targetUrl,
  onTargetChange,
  planName,
  onPlanNameChange,
  plan,
  planning,
  planError,
  onPlan,
  savedPlans,
  onSelectPlan,
  onDeletePlan,
  staticBusy,
  staticCount,
  onGenerateStatic,
  profile,
  mode,
  attackSurface,
  busy,
  canCancel,
  onSelectProfile,
  onSelectMode,
  onSelectAttackSurface,
  documentTemplateFile,
  documentTemplateFlatten,
  onDocumentTemplateFileChange,
  onDocumentTemplateFlattenChange,
  onRun,
  onCancel,
}: AttackFlowProps) {
  const agentSelected = selectedAgentId !== null;
  const generic = selectedAgentId === null;
  const trimmedTarget = targetUrl.trim();
  const targetReady = trimmedTarget !== '';
  // Cosmetic-only check: does the typed value look like a web address? This never
  // gates the run (the real guard is the empty check in `run`); it only powers a
  // gentle, friendly nudge under the field so a non-technical user knows the
  // address is in the right shape.
  const targetLooksLikeUrl = /^https?:\/\/.+/i.test(trimmedTarget);
  const planReady = plan !== null && plan.vectors.length > 0;
  // Step 3 is the terminal action: it stays subordinate until there's something
  // to fire at. Generic runs skip planning, so a target is enough; agent runs are
  // best with a plan but a target alone still fires the default set.
  const canAttack = targetReady && !busy;
  const attackHint = !targetReady
    ? 'Pick an agent or enter its web address above first.'
    : generic
      ? 'Sends the standard set of tricky prompts to the address above.'
      : planReady
        ? `Includes ${plan.vectors.length} attack${plan.vectors.length === 1 ? '' : 's'} tailored to this agent.`
        : 'Tailor attacks above, or just run the standard set now.';

  // Instrument status strip: SCANNING while a job is in flight, ARMED once a
  // target is locked (primed to fire), READY otherwise.
  const consoleState: ConsoleState = busy ? 'scanning' : canAttack ? 'armed' : 'ready';

  return (
    <Card className="w-full max-w-full min-w-0 gap-0 overflow-hidden py-0 shadow-sm">
      {/* Top instrument strip: a thin live status readout above the title bar. */}
      <ConsoleStatusStrip state={consoleState} />

      <header className="flex items-center gap-2 border-b bg-muted/40 px-4 py-3">
        <Swords className="size-4 text-primary" aria-hidden="true" />
        <h2 className="font-mono text-xs font-semibold tracking-[0.12em] uppercase">
          Set up your test
        </h2>
        <span className="ml-auto font-mono text-[11px] tracking-wide text-muted-foreground tabular-nums">
          3 steps
        </span>
      </header>

      <div className="grid gap-0 divide-y">
        {/* 1 · Agent — selecting auto-fills the target endpoint below. */}
        <StepRow
          step={1}
          label="Pick your agent"
          done={agentSelected}
          connectorFilled={agentSelected}
          hint={
            generic
              ? 'No saved agent? Type the web address of the agent you want to test.'
              : 'We filled in its web address for you from its saved settings.'
          }
        >
          <div className="grid gap-2">
            <Label htmlFor="plan-agent" className="sr-only">
              Agent
            </Label>
            <select
              id="plan-agent"
              value={selectedAgentId ?? ''}
              disabled={busy}
              onChange={(e) => onSelectAgent(e.target.value === '' ? null : e.target.value)}
              className="h-9 w-full min-w-0 rounded-md border bg-background px-3 text-sm transition-colors hover:border-foreground/20 focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/50 disabled:opacity-60"
            >
              <option value="">No saved agent — I&apos;ll enter an address</option>
              {agents.map((agent) => (
                <option key={agent.agentId} value={agent.agentId}>
                  {agent.displayName}
                  {agent.hasWorkflow ? ' · workflow' : agent.hasSystemPrompt ? ' · chat' : ''}
                </option>
              ))}
            </select>

            <div className="grid gap-1">
              <div className="flex items-baseline gap-2">
                <Label
                  htmlFor="target-url"
                  className="text-[11px] tracking-wide text-muted-foreground uppercase"
                >
                  Agent URL
                </Label>
                {generic ? null : (
                  <span className="text-[11px] text-muted-foreground/80 normal-case">
                    you can change it
                  </span>
                )}
              </div>
              <div className="relative min-w-0">
                <Target
                  className="pointer-events-none absolute top-1/2 left-2.5 size-3.5 -translate-y-1/2 text-muted-foreground"
                  aria-hidden="true"
                />
                <Input
                  id="target-url"
                  value={targetUrl}
                  onChange={(e) => onTargetChange(e.target.value)}
                  placeholder="e.g. http://127.0.0.1:9102"
                  className="h-8 pl-8 font-mono text-xs"
                  disabled={busy}
                  inputMode="url"
                  aria-describedby="target-url-hint"
                />
              </div>
              {/* Gentle, non-blocking guidance. Validation itself is unchanged —
                  this only reassures or nudges so a non-technical user isn't
                  guessing what belongs here. */}
              <p
                id="target-url-hint"
                className={cn(
                  'text-[11px] leading-snug',
                  targetReady && !targetLooksLikeUrl
                    ? 'text-amber-700 dark:text-amber-400'
                    : 'text-muted-foreground',
                )}
              >
                {!targetReady
                  ? 'The web address where your agent is running. It usually starts with http:// or https://.'
                  : targetLooksLikeUrl
                    ? 'Looks good — this is where we will send the test prompts.'
                    : 'This does not look like a web address yet. It should start with http:// or https://.'}
              </p>
            </div>
          </div>
        </StepRow>

        {/* 2 · Plan — per-agent saved/tailored vectors that seed the run. */}
        <StepRow
          step={2}
          label="Tailor the attacks"
          done={planReady}
          connectorFilled={canAttack}
          optional
          hint={agentSelected ? 'Builds attacks aimed at this specific agent.' : undefined}
        >
          <PlanStep
            agentSelected={agentSelected}
            agentName={connectedAgentName}
            planName={planName}
            onPlanNameChange={onPlanNameChange}
            plan={plan}
            planning={planning}
            planError={planError}
            onPlan={onPlan}
            savedPlans={savedPlans}
            onSelectPlan={onSelectPlan}
            onDeletePlan={onDeletePlan}
            staticBusy={staticBusy}
            staticCount={staticCount}
            onGenerateStatic={onGenerateStatic}
            busy={busy}
          />
        </StepRow>

        {/* 3 · Attack — collapsed options + the one primary CTA. */}
        <StepRow step={3} label="Run the test" terminal hint={attackHint}>
          <div className="grid gap-3">
            <RunOptions
              profile={profile}
              mode={mode}
              attackSurface={attackSurface}
              busy={busy}
              onSelectProfile={onSelectProfile}
              onSelectMode={onSelectMode}
              onSelectAttackSurface={onSelectAttackSurface}
              documentTemplateFile={documentTemplateFile}
              documentTemplateFlatten={documentTemplateFlatten}
              onDocumentTemplateFileChange={onDocumentTemplateFileChange}
              onDocumentTemplateFlattenChange={onDocumentTemplateFlattenChange}
            />

            <div className="flex items-center gap-2">
              {canCancel ? (
                <Button variant="outline" onClick={onCancel}>
                  <X className="size-4" aria-hidden="true" />
                  Cancel
                </Button>
              ) : null}
              <AttackButton state={consoleState} disabled={!canAttack} onClick={onRun} />
            </div>
          </div>
        </StepRow>
      </div>
    </Card>
  );
}

type ConsoleState = 'ready' | 'armed' | 'scanning';

/** The thin top strip of the launch console — a live mono readout (READY / ARMED
 *  / SCANNING) with a state dot. ARMED and SCANNING tint the strip orange so the
 *  whole console reads "live-fire" the moment a target is locked. */
function ConsoleStatusStrip({ state }: { state: ConsoleState }) {
  const map: Record<ConsoleState, { label: string; dot: string; tint: string; text: string }> = {
    ready: {
      label: 'SET ME UP',
      dot: 'bg-muted-foreground/50',
      tint: 'bg-muted/60 border-border',
      text: 'text-muted-foreground',
    },
    armed: {
      label: 'READY TO RUN',
      dot: 'bg-primary',
      tint: 'bg-primary/10 border-primary/30',
      text: 'text-primary',
    },
    scanning: {
      label: 'TESTING',
      dot: 'bg-primary',
      tint: 'bg-primary/10 border-primary/30',
      text: 'text-primary',
    },
  };
  const s = map[state];
  return (
    <div
      className={cn(
        'flex items-center gap-2 border-b px-4 py-1.5 font-mono text-[10px] tracking-[0.22em] uppercase',
        s.tint,
        s.text,
      )}
    >
      <span
        aria-hidden="true"
        className={cn(
          'size-1.5 rounded-full',
          s.dot,
          // Only the live scan pulses; ARMED breathes from the button halo alone,
          // so "armed" doesn't animate from two sources at once.
          state === 'scanning' && 'motion-safe:animate-pulse',
        )}
      />
      <span>{s.label}</span>
      <span className="ml-auto text-muted-foreground/70">tlg · redteam</span>
    </div>
  );
}

/** The single live-fire CTA. ARMED: an orange button that reads `▸ ARM ATTACK`
 *  with a slow breathing ring. SCANNING: the same surface becomes a progress
 *  channel with an indeterminate sweep. READY/disabled: muted and inert. All
 *  pulse/sweep motion is gated behind motion-safe. Accessible name stays
 *  "Attack" so the flow + tests resolve it as the primary action. */
function AttackButton({
  state,
  disabled,
  onClick,
}: {
  state: ConsoleState;
  disabled: boolean;
  onClick: () => void;
}) {
  const scanning = state === 'scanning';
  const armed = state === 'armed';
  return (
    <span className="relative flex flex-1">
      {/* Live-fire breathing halo — an overlay so the button box never repaints.
          Sits behind the button; opacity/transform only, motion-safe gated. */}
      {armed ? (
        <span
          aria-hidden="true"
          className="arm-ring pointer-events-none absolute -inset-px rounded-md bg-primary/40"
        />
      ) : null}
      <button
        type="button"
        onClick={onClick}
        disabled={disabled}
        aria-label="Attack"
        aria-busy={scanning}
        className={cn(
          'group relative flex h-11 flex-1 items-center justify-center gap-2 overflow-hidden rounded-md border text-sm font-semibold tracking-[0.14em] uppercase outline-none transition-[background-color,border-color,transform] duration-200 ease-out',
          'focus-visible:ring-[3px] focus-visible:ring-primary/50',
          'disabled:cursor-not-allowed disabled:border-border disabled:bg-muted disabled:text-muted-foreground',
          armed &&
            'border-primary bg-primary text-primary-foreground shadow-sm hover:bg-primary/90 active:translate-y-px',
          scanning && 'border-primary/60 bg-primary/15 text-primary',
          state === 'ready' && 'border-border bg-muted text-muted-foreground',
        )}
      >
        {/* Indeterminate scan sweep — a light bar traveling the button channel. */}
        {scanning ? (
          <span aria-hidden="true" className="absolute inset-0 overflow-hidden">
            <span className="scan-sweep absolute inset-y-0 left-0 w-1/3 bg-gradient-to-r from-transparent via-primary/35 to-transparent" />
          </span>
        ) : null}
        <span className="relative flex items-center gap-2">
        {scanning ? (
          <>
            <Radar className="size-4 motion-safe:animate-spin" aria-hidden="true" />
            Testing…
          </>
        ) : armed ? (
          <>
            <span aria-hidden="true" className="text-base leading-none">
              ▸
            </span>
            Run test
          </>
        ) : (
          <>
            <Target className="size-4" aria-hidden="true" />
            Run test
          </>
        )}
        </span>
      </button>
    </span>
  );
}

/**
 * One step in the flow: a number badge in a left rail, a label + status, and the
 * step's grouped controls. The rail and consistent padding make the three steps
 * read as one numbered sequence rather than three separate panels.
 */
function StepRow({
  step,
  label,
  children,
  done = false,
  optional = false,
  terminal = false,
  connectorFilled = false,
  hint,
}: {
  step: number;
  label: string;
  children: ReactNode;
  done?: boolean;
  optional?: boolean;
  terminal?: boolean;
  /** When true the connector below this step fills with accent — the checklist
   *  "completes" downward toward the terminal Attack step. */
  connectorFilled?: boolean;
  hint?: string | undefined;
}) {
  return (
    <section className="grid grid-cols-[1.75rem_minmax(0,1fr)] gap-x-3 px-4 py-4">
      {/* Left rail: square step token (terminal aesthetic) + a connector that
          fills with accent once the step is satisfied. */}
      <div className="flex flex-col items-center">
        <span
          className={cn(
            'flex size-7 shrink-0 items-center justify-center rounded-md border font-mono text-xs font-semibold tabular-nums transition-colors duration-200',
            terminal
              ? 'border-primary bg-primary text-primary-foreground'
              : done
                ? 'border-primary/60 bg-primary/10 text-primary'
                : 'border-border bg-background text-muted-foreground',
          )}
          aria-hidden="true"
        >
          {done && !terminal ? <ShieldCheck className="size-3.5" strokeWidth={2.5} /> : step}
        </span>
        {!terminal ? (
          <span className="relative mt-1 w-px flex-1 bg-border" aria-hidden="true">
            <span
              className={cn(
                'absolute inset-0 origin-top bg-primary transition-transform duration-300 ease-out',
                connectorFilled ? 'scale-y-100' : 'scale-y-0',
              )}
            />
          </span>
        ) : null}
      </div>

      <div className="grid min-w-0 gap-2.5">
        <div className="flex min-w-0 flex-wrap items-baseline gap-x-2 gap-y-0.5">
          <span
            className={cn(
              'text-sm font-semibold tracking-tight',
              terminal && 'text-primary',
            )}
          >
            {label}
          </span>
          {optional ? (
            <span className="font-mono text-[10px] tracking-wide text-muted-foreground uppercase">
              optional
            </span>
          ) : null}
          {hint ? (
            <span className="basis-full text-xs leading-snug text-muted-foreground">{hint}</span>
          ) : null}
        </div>
        {children}
      </div>
    </section>
  );
}

/**
 * Run-options stay collapsed by default so the flow reads as 1·2·3 without a flat
 * dump of toggles — the summary line shows the current selection at a glance.
 */
function RunOptions({
  profile,
  mode,
  attackSurface,
  busy,
  onSelectProfile,
  onSelectMode,
  onSelectAttackSurface,
  documentTemplateFile,
  documentTemplateFlatten,
  onDocumentTemplateFileChange,
  onDocumentTemplateFlattenChange,
}: {
  profile: RedteamJobProfile;
  mode: RedteamRunMode;
  attackSurface: RedteamAttackSurface;
  busy: boolean;
  onSelectProfile: (value: RedteamJobProfile) => void;
  onSelectMode: (value: RedteamRunMode) => void;
  onSelectAttackSurface: (value: RedteamAttackSurface) => void;
  documentTemplateFile: File | null;
  documentTemplateFlatten: boolean;
  onDocumentTemplateFileChange: (value: File | null) => void;
  onDocumentTemplateFlattenChange: (value: boolean) => void;
}) {
  return (
    <details className="group w-full max-w-full min-w-0 rounded-md border bg-muted/30 [&_summary::-webkit-details-marker]:hidden">
      <summary className="grid cursor-pointer grid-cols-[auto_auto_minmax(0,1fr)_auto] items-center gap-2 px-3 py-2 text-xs">
        <Sparkles className="size-3.5 text-muted-foreground" aria-hidden="true" />
        <span className="font-medium">Test options</span>
        <span className="min-w-0 truncate font-mono text-[11px] text-muted-foreground">
          {profile} · {mode === 'one_off' ? 'one-off' : 'learning'} ·{' '}
          {attackSurface === 'chat' ? 'chat' : 'document'}
        </span>
        <ChevronDown
          className="size-3.5 text-muted-foreground transition-transform group-open:rotate-180 motion-reduce:transition-none"
          aria-hidden="true"
        />
      </summary>
      <div className="grid gap-3 border-t p-3">
        <OptionGroup
          label="How much"
          options={REDTEAM_JOB_PROFILES}
          selected={profile}
          busy={busy}
          onSelect={onSelectProfile}
          render={(value) => value}
          caption={PROFILE_COPY[profile]}
        />
        <OptionGroup
          label="Style"
          options={REDTEAM_RUN_MODES}
          selected={mode}
          busy={busy}
          onSelect={onSelectMode}
          render={(value) => (value === 'one_off' ? 'one-off' : 'learning')}
          caption={MODE_COPY[mode]}
        />
        <OptionGroup
          label="Where"
          options={REDTEAM_ATTACK_SURFACES}
          selected={attackSurface}
          busy={busy}
          onSelect={onSelectAttackSurface}
          render={(value) => (value === 'chat' ? 'chat' : 'document')}
          caption={SURFACE_COPY[attackSurface]}
        />
        {attackSurface === 'document_workflow' ? (
          <div className="grid gap-2 rounded-md border bg-background p-3">
            <div className="grid gap-1">
              <span className="flex items-center gap-1.5 font-mono text-[10px] tracking-wide text-muted-foreground uppercase">
                Hidden-message form test
                <InfoHint label="How the document test works">
                  Your PDF needs fillable boxes (an AcroForm). We put fake values in
                  fields like name, SSN, and amount, and slip the hidden instruction
                  into a free-text field such as notes or address.
                </InfoHint>
              </span>
              <p className="text-xs text-muted-foreground">
                Upload a blank PDF form — we fill it with fake personal details and
                hide a sneaky instruction inside, then hand it to your agent to see
                if it gets tricked. Leave it empty and we&apos;ll make a fake form for
                you instead.
              </p>
            </div>
            <div className="grid gap-1.5">
              <Label htmlFor="document-template" className="text-xs">
                Your PDF form <span className="font-normal text-muted-foreground">· optional</span>
              </Label>
              <Input
                id="document-template"
                type="file"
                accept="application/pdf,.pdf"
                disabled={busy}
                onChange={(event) =>
                  onDocumentTemplateFileChange(event.currentTarget.files?.[0] ?? null)
                }
              />
              <span className="truncate text-[11px] text-muted-foreground">
                {documentTemplateFile
                  ? documentTemplateFile.name
                  : 'Use a form with boxes you can type into.'}
              </span>
            </div>
            <label className="flex items-center gap-2 text-xs">
              <input
                type="checkbox"
                checked={documentTemplateFlatten}
                disabled={busy || documentTemplateFile === null}
                onChange={(event) => onDocumentTemplateFlattenChange(event.target.checked)}
              />
              <span className="flex items-center gap-1.5">
                Make it look printed or scanned
                <InfoHint label="What “printed or scanned” means">
                  Flattens the form so it reads like a static printout instead of a
                  fillable PDF — closer to a document a real person would send.
                </InfoHint>
              </span>
            </label>
          </div>
        ) : null}
      </div>
    </details>
  );
}

/** A labelled row of segmented toggle buttons with a one-line caption. */
function OptionGroup<T extends string>({
  label,
  options,
  selected,
  busy,
  onSelect,
  render,
  caption,
}: {
  label: string;
  options: readonly T[];
  selected: T;
  busy: boolean;
  onSelect: (value: T) => void;
  render: (value: T) => string;
  caption: string;
}) {
  return (
    <div className="grid gap-1.5">
      <div className="flex min-w-0 flex-wrap items-center gap-2">
        <span className="w-14 shrink-0 font-mono text-[10px] tracking-wide text-muted-foreground uppercase">
          {label}
        </span>
        <div className="inline-flex max-w-full flex-wrap overflow-hidden rounded-md border">
          {options.map((value, index) => (
            <button
              key={value}
              type="button"
              aria-pressed={value === selected}
              disabled={busy}
              onClick={() => onSelect(value)}
              className={cn(
                // Neutral filled selection — orange (bg-primary) is reserved for
                // the single Attack CTA, so these toggles never compete with it.
                'min-w-0 px-3 py-1 text-xs font-semibold uppercase transition-colors disabled:opacity-60',
                index > 0 && 'border-l',
                value === selected
                  ? 'bg-foreground text-background'
                  : 'bg-background hover:bg-muted',
              )}
            >
              {render(value)}
            </button>
          ))}
        </div>
      </div>
      <span className="min-w-0 pl-16 text-[11px] leading-snug text-muted-foreground">{caption}</span>
    </div>
  );
}

async function buildDocumentTemplate({
  file,
  flatten,
}: {
  file: File;
  flatten: boolean;
}): Promise<DocumentTemplateInput> {
  if (file.size > MAX_DOCUMENT_TEMPLATE_BYTES) {
    throw new Error('PDF form must be 10 MB or smaller.');
  }
  const isPdf = file.name.toLowerCase().endsWith('.pdf') || file.type === 'application/pdf';
  if (!isPdf) {
    throw new Error('PDF form must be a .pdf file.');
  }
  const buffer = await file.arrayBuffer();
  const bytes = new Uint8Array(buffer);
  if (
    bytes.length < 5 ||
    bytes[0] !== 0x25 ||
    bytes[1] !== 0x50 ||
    bytes[2] !== 0x44 ||
    bytes[3] !== 0x46 ||
    bytes[4] !== 0x2d
  ) {
    throw new Error('PDF form must contain PDF bytes.');
  }
  return {
    fileName: file.name,
    mediaType: file.type || 'application/pdf',
    dataBase64: bytesToBase64(bytes),
    flatten,
  };
}

function bytesToBase64(bytes: Uint8Array): string {
  // Build the binary string in chunks: a one-char-at-a-time concat reallocates
  // per byte (UI jank near the 10 MB cap), and spreading the whole array into
  // String.fromCharCode overflows the call stack.
  const CHUNK = 0x8000;
  let binary = '';
  for (let i = 0; i < bytes.length; i += CHUNK) {
    binary += String.fromCharCode(...bytes.subarray(i, i + CHUNK));
  }
  return btoa(binary);
}

const EMPTY_STEPS: ReadonlyArray<{ icon: typeof Target; title: string; body: string }> = [
  {
    icon: Target,
    title: 'Pick your agent',
    body: 'Choose a saved agent, or type the web address of the agent you want to test.',
  },
  {
    icon: Crosshair,
    title: 'Tailor the attacks (optional)',
    body: 'Build prompts aimed at your specific agent, or skip this and use the standard set.',
  },
  {
    icon: Swords,
    title: 'Run the test',
    body: 'We send the tricky prompts and check each reply. Your results and a suggested fix appear right here.',
  },
];

/** The right pane before any plan or run: a "threat board standing by" — the
 *  board chrome is present but idle (a dim radar header), and it still teaches the
 *  three-step sequence so a first-timer knows what fills it. */
function DetailEmptyState({ hasHistory }: { hasHistory: boolean }) {
  return (
    <section
      aria-label="No results yet"
      className="min-w-0 overflow-hidden rounded-xl border border-dashed bg-card/40 shadow-sm"
    >
      <header className="flex min-w-0 flex-wrap items-center gap-x-3 gap-y-1 border-b border-dashed bg-muted/30 px-4 py-2.5">
        <Radar className="size-4 text-muted-foreground" aria-hidden="true" />
        <h2 className="font-mono text-[11px] font-semibold tracking-[0.15em] text-muted-foreground uppercase">
          Results
        </h2>
        <span className="ml-auto min-w-0 truncate font-mono text-[11px] tracking-[0.18em] text-muted-foreground/70 uppercase">
          waiting to start
        </span>
      </header>

      <div className="grid min-w-0 gap-4 p-4 lg:p-5">
        <p className="min-w-0 text-sm leading-relaxed text-muted-foreground">
          {hasHistory
            ? 'No test running right now. Pick a past test on the left to revisit it, or start a new one — your results will show up here.'
            : 'You haven’t run a test yet. Follow the three steps on the left to start one. Your results — and a suggested fix for anything that gets through — will appear here.'}
        </p>

        <ol className="grid gap-2">
          {EMPTY_STEPS.map((item, index) => (
            <li
              key={item.title}
              className="grid min-w-0 grid-cols-[1.75rem_auto_minmax(0,1fr)] items-start gap-3 rounded-lg border bg-card p-3 transition-colors hover:border-foreground/15"
            >
              <span className="flex size-7 items-center justify-center rounded-md border border-primary/40 bg-primary/10 font-mono text-xs font-semibold text-primary tabular-nums">
                {index + 1}
              </span>
              <item.icon className="mt-1 size-4 text-muted-foreground" aria-hidden="true" />
              <div className="grid min-w-0 gap-0.5">
                <p className="text-sm font-medium">{item.title}</p>
                <p className="min-w-0 text-xs leading-relaxed text-muted-foreground">{item.body}</p>
              </div>
            </li>
          ))}
        </ol>
      </div>
    </section>
  );
}

/** Live scanning state: the board with shimmer skeleton threat rows instead of a
 *  bare spinner. The SCANNING strip + a radar sweep make it read as "in progress"
 *  while the server runs the job. */
function ScanningBoard({ target }: { target: string }) {
  return (
    <section
      aria-label="Testing your agent"
      aria-busy="true"
      className="min-w-0 overflow-hidden rounded-xl border border-primary/30 bg-card shadow-sm ring-1 ring-primary/10"
    >
      <header className="relative flex min-w-0 flex-wrap items-center gap-x-3 gap-y-1 overflow-hidden border-b bg-primary/[0.06] px-4 py-2.5">
        <Radar className="size-4 text-primary motion-safe:animate-spin" aria-hidden="true" />
        <h2 className="font-mono text-[11px] font-semibold tracking-[0.15em] text-primary uppercase">
          Testing…
        </h2>
        <span className="ml-auto min-w-0 truncate font-mono text-[11px] text-muted-foreground">
          {target}
        </span>
        {/* A sweep crossing the header to signal live work. */}
        <span aria-hidden="true" className="pointer-events-none absolute inset-0 overflow-hidden">
          <span className="scan-sweep absolute inset-y-0 left-0 w-1/4 bg-gradient-to-r from-transparent via-primary/15 to-transparent" />
        </span>
      </header>
      <p className="border-b bg-primary/[0.03] px-4 py-2 text-xs leading-relaxed text-muted-foreground">
        We&apos;re sending tricky prompts to your agent and checking each reply. This can
        take a minute or two — it&apos;s safe to leave this page and come back; the
        results will be saved.
      </p>
      <ul className="grid gap-px bg-border/50 sm:grid-cols-2">
        {[0, 1, 2, 3].map((i) => (
          <li key={i} className="relative flex gap-3 overflow-hidden bg-card px-3 py-3">
            <span aria-hidden="true" className="w-1 shrink-0 self-stretch rounded-full bg-border" />
            <div className="grid min-w-0 flex-1 gap-2">
              <div className="h-3 w-24 rounded bg-muted" />
              <div className="h-2.5 w-16 rounded bg-muted/70" />
              <div className="h-3 w-full rounded bg-muted/60" />
            </div>
            {/* Shimmer pass over each skeleton row. */}
            <span
              aria-hidden="true"
              className="scan-shimmer absolute inset-0 bg-gradient-to-r from-transparent via-foreground/[0.04] to-transparent"
            />
          </li>
        ))}
      </ul>
    </section>
  );
}

/** The headline verdict readout — a console gauge. The big tabular percentage is
 *  the single loudest number on the page; orange/red when anything landed, calm
 *  green when the agent held. A plain-language verdict sentence + a "what next"
 *  line make the number meaningful to a non-technical reader. A thin segmented bar
 *  splits landed vs blocked. */
function ResultSummary({ job }: { job: RedteamJobSummary }) {
  const percent = landedPercent(job);
  const done = isTerminalStatus(job.status);
  const breached = percent > 0;
  const total = job.landed + job.blocked;
  const landedShare = total > 0 ? (job.landed / total) * 100 : 0;
  // Verdict semantics: a breach reads in the sacred BLOCK color, a held agent in
  // the ALLOW color — never an ad-hoc red/green, so the readout matches verdict
  // badges everywhere else.
  const verdictColor = breached ? 'var(--color-block)' : 'var(--color-allow)';

  // Plain-language verdict: spell out whether this number is good or bad, and what
  // to do next, so a non-technical reader never has to interpret a raw percentage.
  const headline = !done
    ? 'Running the test…'
    : breached
      ? `${job.landed} of ${job.attacks} tricky ${job.attacks === 1 ? 'prompt' : 'prompts'} got past your guardrails.`
      : `Great — your guardrails caught all ${job.attacks} tricky ${job.attacks === 1 ? 'prompt' : 'prompts'}.`;
  const nextStep = !done
    ? null
    : breached
      ? 'Lower is better. Open each result below to see what got through, then add the suggested guard and re-run to confirm it is fixed.'
      : 'Nothing got through. You can share this result, or run a more thorough test from the options on the left.';

  return (
    <section className="min-w-0 overflow-hidden rounded-xl border bg-card shadow-sm ring-1 ring-border/60">
      <header className="flex items-center gap-2 border-b bg-muted/40 px-4 py-2.5">
        <ShieldAlert className="size-3.5" style={{ color: verdictColor }} aria-hidden="true" />
        <span className="font-mono text-[11px] font-semibold tracking-[0.15em] uppercase">
          Result
        </span>
        <span className="ml-auto">
          <StatusBadge status={job.status} />
        </span>
      </header>

      <div className="grid gap-3 p-4 lg:p-5">
        <div className="flex flex-wrap items-end justify-between gap-4">
          <div className="flex items-end gap-2">
            <span
              className="font-mono text-5xl leading-none font-semibold tabular-nums"
              style={{ color: verdictColor }}
            >
              {done ? `${percent}%` : '—'}
            </span>
            <span className="flex items-center gap-1 pb-1 text-sm text-muted-foreground">
              {breached ? 'attacks landed' : 'got through'}
              <InfoHint>
                The share of tricky prompts that got past your guardrails. Lower is
                better — 0% means everything was caught.
              </InfoHint>
            </span>
          </div>
          <p className="font-mono text-[11px] tracking-wide text-muted-foreground tabular-nums uppercase">
            {done
              ? `${job.landed}/${job.attacks} landed${job.blocked > 0 ? ` · ${job.blocked} blocked` : ''}`
              : 'running…'}
          </p>
        </div>

        {done && total > 0 ? (
          <div
            className="flex h-2 w-full overflow-hidden rounded-full"
            style={{ backgroundColor: 'color-mix(in oklab, var(--color-allow), transparent 75%)' }}
            role="presentation"
            aria-hidden="true"
          >
            <span
              className="h-full rounded-full"
              style={{ width: `${landedShare}%`, backgroundColor: 'var(--color-block)' }}
            />
          </div>
        ) : null}

        <p className="text-sm font-medium text-foreground" style={{ color: done ? verdictColor : undefined }}>
          {headline}
        </p>
        {nextStep ? <p className="text-sm leading-relaxed text-muted-foreground">{nextStep}</p> : null}
      </div>
    </section>
  );
}

/** After a run the Threat Board flips to outcomes — same chrome as PlanVectors,
 *  now graded by what actually happened: landed attacks get hot/red emphasis +
 *  a left accent bar and sort first; held attacks read calm green with a verdict
 *  pip. Each row expands to its adversarial prompt + the agent's reply. */
function ThreatResultBoard({
  results,
  expanded,
  onToggle,
}: {
  results: readonly RedteamJobResult[];
  expanded: number | null;
  onToggle: (index: number | null) => void;
}) {
  // Landed first — the operator wants the breaches at the top of the board.
  const ordered = results
    .map((item, index) => ({ item, index }))
    .sort((a, b) => Number(b.item.landed) - Number(a.item.landed));

  const landed = results.filter((r) => r.landed).length;

  return (
    <section
      aria-label="Every prompt we tried"
      className="min-w-0 overflow-hidden rounded-xl border bg-card shadow-sm ring-1 ring-border/60"
    >
      <header className="flex min-w-0 flex-wrap items-center gap-x-3 gap-y-1 border-b bg-muted/40 px-4 py-2.5">
        <Radar className="size-4 text-primary" aria-hidden="true" />
        <h2 className="font-mono text-[11px] font-semibold tracking-[0.15em] uppercase">
          Every prompt we tried
        </h2>
        <span className="ml-auto min-w-0 truncate font-mono text-[11px] tabular-nums text-muted-foreground">
          <span style={{ color: landed > 0 ? 'var(--color-block)' : 'var(--color-allow)' }}>
            {landed}
          </span>{' '}
          / {results.length} got through
        </span>
      </header>

      <p className="border-b px-4 py-2 text-xs leading-relaxed text-muted-foreground">
        Red rows{' '}
        <span className="font-medium" style={{ color: 'var(--color-block)' }}>
          got past
        </span>{' '}
        your guardrails; green rows were{' '}
        <span className="font-medium" style={{ color: 'var(--color-allow)' }}>
          caught
        </span>
        . Click any row to read the prompt we sent and how your agent replied.
      </p>

      <ul className="grid gap-px bg-border/50 sm:grid-cols-2">
        {ordered.map(({ item, index }) => {
          const open = expanded === index;
          const evidenceId = `attack-evidence-${index}`;
          const breached = item.landed;
          return (
            <li key={`${item.attack}-${item.seq}`} className="grid bg-card">
              <button
                type="button"
                onClick={() => onToggle(open ? null : index)}
                aria-expanded={open}
                aria-controls={evidenceId}
                className={cn(
                  'group flex w-full min-w-0 items-center gap-3 px-3 py-2.5 text-left transition-colors duration-150 ease-out',
                  breached ? 'hover:bg-destructive/5' : 'hover:bg-muted/40',
                )}
              >
                <span
                  aria-hidden="true"
                  className="w-1 shrink-0 self-stretch rounded-full"
                  style={{
                    backgroundColor: breached
                      ? 'var(--color-block)'
                      : 'color-mix(in oklab, var(--color-allow), transparent 35%)',
                  }}
                />
                <span className="grid min-w-0 flex-1 gap-1">
                  <span className="flex items-center gap-2">
                    <OutcomeBadge outcome={item.outcome} landed={item.landed} />
                    <span className="truncate font-mono text-[11px] text-muted-foreground">
                      {item.attack}
                    </span>
                  </span>
                  <span className="truncate text-sm text-foreground/90">{item.goal}</span>
                </span>
                <ChevronDown
                  aria-hidden="true"
                  className={cn(
                    'size-4 shrink-0 text-muted-foreground transition-transform duration-150 ease-out motion-reduce:transition-none',
                    open && 'rotate-180',
                  )}
                />
              </button>
              <div
                id={evidenceId}
                hidden={!open}
                className="grid gap-3 border-t bg-muted/30 px-3 pb-4"
              >
                {item.prompt ? <Evidence title="The prompt we sent" body={item.prompt} /> : null}
                <Evidence title="How your agent replied" body={item.reply} traceId={item.trace_id ?? null} />
              </div>
            </li>
          );
        })}
      </ul>
    </section>
  );
}

function JobHistory({
  jobs,
  activeId,
  onSelect,
}: {
  jobs: readonly RedteamJobSummary[];
  activeId: string | null;
  onSelect: (id: string) => void;
}) {
  return (
    <Card className="min-w-0 overflow-hidden gap-0 p-0 py-0 shadow-sm">
      <CardHeader className="flex flex-row items-center gap-2 border-b bg-muted/40 px-4 py-2.5">
        <Clock className="size-3.5 text-muted-foreground" aria-hidden="true" />
        <CardTitle className="font-mono text-[11px] font-semibold tracking-[0.15em] uppercase">
          Past tests
        </CardTitle>
        <span className="ml-auto font-mono text-[11px] text-muted-foreground tabular-nums">
          {jobs.length}
        </span>
      </CardHeader>
      <ul className="divide-y">
        {jobs.map((item) => {
          const active = item.id === activeId;
          return (
            <li key={item.id} className="relative">
              {/* Active marker rail — the orange tab on the selected job. */}
              {active ? (
                <span
                  aria-hidden="true"
                  className="absolute inset-y-0 left-0 w-0.5 bg-primary"
                />
              ) : null}
              <button
                type="button"
                onClick={() => onSelect(item.id)}
                aria-current={active ? 'true' : undefined}
                className={cn(
                  'grid w-full min-w-0 grid-cols-[minmax(0,1fr)_auto_auto] items-center gap-3 px-4 py-2.5 text-left transition-colors duration-150 hover:bg-muted/60',
                  active && 'bg-primary/[0.06]',
                )}
              >
                <span className="truncate font-mono text-xs text-muted-foreground">
                  {item.target}
                </span>
                <span
                  className="font-mono text-xs text-muted-foreground tabular-nums"
                  style={
                    isTerminalStatus(item.status) && item.landed > 0
                      ? { color: 'var(--color-block)' }
                      : undefined
                  }
                >
                  {isTerminalStatus(item.status) ? `${item.landed}/${item.attacks}` : '—'}
                </span>
                <StatusBadge status={item.status} />
              </button>
            </li>
          );
        })}
      </ul>
    </Card>
  );
}

function Evidence({
  title,
  body,
  traceId,
}: {
  title: string;
  body: string;
  traceId?: string | null;
}) {
  return (
    <div className="grid gap-1.5 pt-3">
      <div className="font-mono text-[11px] tracking-wide text-muted-foreground uppercase">
        {title}
      </div>
      <div className="rounded-md border bg-background px-3 py-2 text-sm leading-6">{body}</div>
      {traceId ? (
        <div className="font-mono text-[11px] break-all text-muted-foreground">
          tlg.trace_id = {traceId}
        </div>
      ) : null}
    </div>
  );
}

function OutcomeBadge({ outcome, landed }: { outcome: string; landed?: boolean }) {
  // Outcome → sacred verdict color: a landed attack is a breach (BLOCK token, the
  // guard that should have fired); a held attack reads in the ALLOW token. Error
  // stays neutral. Using the Badge verdict variants keeps these legible in both
  // themes and consistent with verdict chips elsewhere.
  if (outcome === 'landed' || landed) {
    return (
      <Badge variant="block" className="gap-1 font-mono text-[10px] tracking-wider uppercase">
        <ShieldAlert className="size-3" aria-hidden="true" />
        landed
      </Badge>
    );
  }
  if (outcome === 'error') {
    return (
      <Badge variant="outline" className="font-mono text-[10px] tracking-wider uppercase">
        error
      </Badge>
    );
  }
  return (
    <Badge variant="allow" className="gap-1 font-mono text-[10px] tracking-wider uppercase">
      <ShieldCheck className="size-3" aria-hidden="true" />
      {outcome === 'clean' ? 'blocked' : outcome}
    </Badge>
  );
}

/** Process-state tone (NOT a verdict). The sacred verdict tokens are reserved for
 *  guardrail verdicts, so job lifecycle uses neutral tones: running pulls the brand
 *  accent so an in-flight job stands out, error uses the destructive token, and the
 *  rest stay muted (the status text carries the meaning). */
const STATUS_STYLE: Record<JobStatus, string | undefined> = {
  queued: undefined,
  running: 'var(--primary)',
  complete: undefined,
  error: 'var(--destructive)',
  cancelled: undefined,
};

function StatusBadge({ status }: { status: JobStatus }) {
  const color = STATUS_STYLE[status];
  return (
    <Badge
      variant="outline"
      className="font-mono text-[10px] tracking-[0.12em] uppercase"
      style={color ? { color } : undefined}
    >
      {status}
    </Badge>
  );
}
