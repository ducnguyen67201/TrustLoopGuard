'use client';

import { Activity, ChevronDown, Play, ShieldAlert, ShieldCheck, Swords } from 'lucide-react';
import { useCallback, useEffect, useRef, useState } from 'react';

import {
  REDTEAM_PROFILES,
  successPercent,
  type RedteamCase,
  type RedteamOutcome,
} from '@/lib/arena-redteam';
import {
  pollAttackRun,
  startAttackRun,
  type RedteamProfile,
  type RedteamReport,
} from '@/lib/attacks';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { cn } from '@/lib/utils';

type PanelState = 'idle' | 'starting' | 'running' | 'complete' | 'error';

const POLL_INTERVAL_MS = 1200;
const DEFAULT_TARGET = 'http://127.0.0.1:9102';

const PROFILE_COPY: Record<RedteamProfile, string> = {
  fast: 'A few attacks — quick check',
  full: 'Every attack class',
  max: 'Attack × phrasing sweep',
};

const ADAPTER_SNIPPET = `import { createArenaAdapter } from './arena/adapter';

await createArenaAdapter({
  host: '127.0.0.1',
  port: 9102,
  profile, // { displayName, systemPrompt, safeUserQuestion, protectedInformationName }
  async chat({ message }) {
    const reply = await myAgent(message);
    return { content: reply, finishReason: 'stop', verdict: null, phase: null, traceId: null };
  },
});`;

const delay = (ms: number) => new Promise<void>((resolve) => setTimeout(resolve, ms));

function messageOf(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

export function AttacksPanel() {
  const [targetUrl, setTargetUrl] = useState(DEFAULT_TARGET);
  const [profile, setProfile] = useState<RedteamProfile>('fast');
  const [state, setState] = useState<PanelState>('idle');
  const [report, setReport] = useState<RedteamReport | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [expanded, setExpanded] = useState<number | null>(null);

  const cancelledRef = useRef(false);
  useEffect(
    () => () => {
      cancelledRef.current = true;
    },
    [],
  );

  const busy = state === 'starting' || state === 'running';

  const run = useCallback(async () => {
    const target = targetUrl.trim();
    if (target === '') {
      setError('Enter your agent URL first.');
      return;
    }
    setState('starting');
    setError(null);
    setReport(null);
    setExpanded(null);
    cancelledRef.current = false;

    let runId: string;
    try {
      runId = (await startAttackRun({ profile, targetUrl: target })).runId;
    } catch (err) {
      setState('error');
      setError(messageOf(err));
      return;
    }

    setState('running');
    while (!cancelledRef.current) {
      try {
        const poll = await pollAttackRun(runId);
        if (poll.report) setReport(poll.report);
        if (poll.status === 'complete') {
          setState('complete');
          return;
        }
        if (poll.status === 'error') {
          setState('error');
          setError(poll.report?.error ?? 'the attack run failed');
          return;
        }
      } catch (err) {
        setState('error');
        setError(messageOf(err));
        return;
      }
      await delay(POLL_INTERVAL_MS);
    }
  }, [profile, targetUrl]);

  const attacks = report?.cases.filter((c) => !c.control) ?? [];

  return (
    <div className="mx-auto grid w-full max-w-4xl gap-6 p-4 lg:p-6">
      <div className="grid gap-1">
        <h1 className="flex items-center gap-2 text-2xl font-semibold tracking-tight">
          <Swords className="size-6 text-primary" />
          Attack an agent
        </h1>
        <p className="text-sm text-muted-foreground">
          Point an independent red-team at one agent endpoint and see which attacks get through.
        </p>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Target</CardTitle>
          <CardDescription>
            Your agent must expose the arena adapter contract. Local loopback only (127.0.0.1 /
            localhost).
          </CardDescription>
        </CardHeader>
        <CardContent className="grid gap-4">
          <div className="grid gap-2">
            <Label htmlFor="target-url">Agent URL</Label>
            <Input
              id="target-url"
              value={targetUrl}
              onChange={(e) => setTargetUrl(e.target.value)}
              placeholder={DEFAULT_TARGET}
              className="font-mono"
              disabled={busy}
            />
          </div>

          <details className="rounded-md border bg-muted/40 text-sm">
            <summary className="cursor-pointer list-none px-3 py-2 font-medium">
              How to expose your agent
            </summary>
            <pre className="overflow-x-auto border-t px-3 py-2 text-xs leading-5">
              {ADAPTER_SNIPPET}
            </pre>
          </details>

          <div className="flex flex-wrap items-center justify-between gap-3">
            <div className="flex flex-wrap items-center gap-2">
              {REDTEAM_PROFILES.map((value) => (
                <button
                  key={value}
                  type="button"
                  aria-pressed={value === profile}
                  disabled={busy}
                  onClick={() => setProfile(value)}
                  className={cn(
                    'rounded-md border px-3 py-1.5 text-xs font-semibold uppercase transition-colors disabled:opacity-60',
                    value === profile
                      ? 'border-primary bg-primary text-primary-foreground'
                      : 'bg-background hover:bg-accent',
                  )}
                >
                  {value}
                </button>
              ))}
              <span className="text-xs text-muted-foreground">{PROFILE_COPY[profile]}</span>
            </div>
            <Button onClick={() => void run()} disabled={busy}>
              {busy ? <Activity className="size-4 animate-pulse" /> : <Play className="size-4" />}
              {busy ? 'Attacking…' : 'Attack'}
            </Button>
          </div>
        </CardContent>
      </Card>

      {error ? (
        <p className="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
          {error}
        </p>
      ) : null}

      {report ? (
        <ResultSummary report={report} running={state === 'running'} />
      ) : busy ? (
        <p className="text-sm text-muted-foreground">Attacking {targetUrl}…</p>
      ) : null}

      {attacks.length > 0 ? (
        <AttackList cases={attacks} expanded={expanded} onToggle={setExpanded} />
      ) : null}
    </div>
  );
}

function ResultSummary({ report, running }: { report: RedteamReport; running: boolean }) {
  const percent = successPercent(report.guarded);
  const { landed, attacks, blocked, errored } = report.guarded;
  return (
    <Card>
      <CardContent className="flex flex-wrap items-end justify-between gap-4 pt-6">
        <div className="grid gap-1">
          <div className="flex items-end gap-2">
            <span
              className={cn(
                'text-5xl font-semibold tabular-nums',
                percent > 0 ? 'text-destructive' : 'text-emerald-600',
              )}
            >
              {percent}%
            </span>
            <span className="pb-2 text-sm text-muted-foreground">attacks landed</span>
          </div>
          <p className="text-sm text-muted-foreground">
            {landed} / {attacks} attacks landed
            {blocked > 0 ? ` · ${blocked} blocked` : ''}
            {errored > 0 ? ` · ${errored} errored` : ''}
          </p>
        </div>
        <div className="text-xs text-muted-foreground">
          engine <span className="font-mono">{report.llm.mode}</span>
          {running ? (
            <span className="ml-2">
              · {report.progress.done}/{report.progress.total} campaigns
            </span>
          ) : null}
        </div>
      </CardContent>
    </Card>
  );
}

function AttackList({
  cases,
  expanded,
  onToggle,
}: {
  cases: readonly RedteamCase[];
  expanded: number | null;
  onToggle: (index: number | null) => void;
}) {
  return (
    <Card className="overflow-hidden p-0">
      <ul className="divide-y">
        {cases.map((item, index) => {
          const open = expanded === index;
          const evidenceId = `attack-evidence-${index}`;
          return (
            <li key={`${item.attack}-${index}`}>
              <button
                type="button"
                onClick={() => onToggle(open ? null : index)}
                aria-expanded={open}
                aria-controls={evidenceId}
                className="grid w-full grid-cols-[1fr_auto_2rem] items-center gap-3 px-4 py-3 text-left hover:bg-accent"
              >
                <span className="grid">
                  <span className="font-mono text-sm">{item.attack}</span>
                  <span className="truncate text-xs text-muted-foreground">{item.goal}</span>
                </span>
                <OutcomeBadge outcome={item.guarded.outcome} />
                <ChevronDown
                  className={cn(
                    'size-4 text-muted-foreground transition-transform motion-reduce:transition-none',
                    open && 'rotate-180',
                  )}
                />
              </button>
              <div id={evidenceId} hidden={!open} className="grid gap-3 bg-muted/40 px-4 pb-4">
                {item.prompt ? <Evidence title="Adversarial prompt" body={item.prompt} /> : null}
                <Evidence
                  title="Agent reply"
                  body={item.guarded.reply || item.guarded.detail}
                  traceId={item.guarded.traceId}
                />
              </div>
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
    <div className="grid gap-1 pt-3">
      <div className="font-mono text-[11px] text-muted-foreground uppercase">{title}</div>
      <div className="rounded-md border bg-background px-3 py-2 text-sm leading-6">{body}</div>
      {traceId ? (
        <div className="font-mono text-[11px] break-all text-muted-foreground">
          tlg.trace_id = {traceId}
        </div>
      ) : null}
    </div>
  );
}

const OUTCOME_LABEL: Record<RedteamOutcome, string> = {
  landed: 'landed',
  blocked: 'blocked',
  clean: 'safe',
  error: 'error',
};

function OutcomeBadge({ outcome }: { outcome: RedteamOutcome }) {
  if (outcome === 'landed') {
    return (
      <Badge variant="destructive" className="gap-1">
        <ShieldAlert className="size-3" />
        {OUTCOME_LABEL.landed}
      </Badge>
    );
  }
  if (outcome === 'error') {
    return <Badge variant="outline">{OUTCOME_LABEL.error}</Badge>;
  }
  return (
    <Badge variant="outline" className="gap-1 border-emerald-500/50 text-emerald-600">
      <ShieldCheck className="size-3" />
      {OUTCOME_LABEL[outcome]}
    </Badge>
  );
}
