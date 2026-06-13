'use client';

import {
  Activity,
  ChevronDown,
  Clock,
  Play,
  ShieldAlert,
  ShieldCheck,
  Swords,
  X,
} from 'lucide-react';
import { useCallback, useEffect, useRef, useState } from 'react';

import {
  REDTEAM_JOB_PROFILES,
  cancelJob,
  dispatchJob,
  getJob,
  isTerminalStatus,
  listJobs,
  type JobStatus,
  type RedteamJobProfile,
  type RedteamJobResult,
  type RedteamJobSummary,
} from '@/lib/redteam-jobs';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { cn } from '@/lib/utils';

import { HardenJobCard } from './harden-job-card';

const POLL_INTERVAL_MS = 1200;
const DEFAULT_TARGET = 'http://127.0.0.1:9102';
const HISTORY_LIMIT = 10;

const PROFILE_COPY: Record<RedteamJobProfile, string> = {
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

function landedPercent(job: RedteamJobSummary): number {
  return job.attacks > 0 ? Math.round((job.landed / job.attacks) * 100) : 0;
}

export function AttacksPanel() {
  const [targetUrl, setTargetUrl] = useState(DEFAULT_TARGET);
  const [profile, setProfile] = useState<RedteamJobProfile>('fast');
  const [job, setJob] = useState<RedteamJobSummary | null>(null);
  const [results, setResults] = useState<RedteamJobResult[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [expanded, setExpanded] = useState<number | null>(null);
  const [dispatching, setDispatching] = useState(false);
  const [history, setHistory] = useState<RedteamJobSummary[]>([]);

  // The id currently being polled. Setting it to null breaks the poll loop —
  // used by cancel, unmount, and starting/loading a different job.
  const activeJobRef = useRef<string | null>(null);
  useEffect(
    () => () => {
      activeJobRef.current = null;
    },
    [],
  );

  const refreshHistory = useCallback(async () => {
    try {
      setHistory(await listJobs({ limit: HISTORY_LIMIT }));
    } catch {
      // History is best-effort; a failure here must not break the active run.
    }
  }, []);

  useEffect(() => {
    void refreshHistory();
  }, [refreshHistory]);

  const busy = dispatching || (job !== null && !isTerminalStatus(job.status));

  // A finished run's report (summary, evidence, harden card, error) must not
  // linger under a new configuration. Editing the target or switching profiles
  // drops the stale view and stops any poll. Inputs are disabled while busy, so
  // this only fires between runs. Guarded so per-keystroke edits are a no-op once
  // already cleared.
  const clearStaleRun = () => {
    if (job === null && error === null) return;
    activeJobRef.current = null;
    setJob(null);
    setResults([]);
    setError(null);
    setExpanded(null);
  };

  const poll = useCallback(
    async (id: string) => {
      while (activeJobRef.current === id) {
        let detail;
        try {
          detail = await getJob(id);
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
    setError(null);
    setJob(null);
    setResults([]);
    setExpanded(null);

    let summary: RedteamJobSummary;
    try {
      summary = await dispatchJob({ targetUrl: target, profile });
    } catch (err) {
      setDispatching(false);
      setError(messageOf(err));
      return;
    }
    setJob(summary);
    setDispatching(false);
    activeJobRef.current = summary.id;
    await poll(summary.id);
  }, [profile, targetUrl, poll]);

  const cancel = useCallback(async () => {
    if (job === null) return;
    activeJobRef.current = null;
    try {
      setJob(await cancelJob(job.id));
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
        const detail = await getJob(id);
        setJob(detail.job);
        setResults(detail.results);
        if (!isTerminalStatus(detail.job.status)) {
          activeJobRef.current = id;
          await poll(id);
        }
      } catch (err) {
        setError(messageOf(err));
      }
    },
    [poll],
  );

  return (
    <div className="mx-auto grid w-full max-w-4xl gap-6 p-4 lg:p-6">
      <div className="grid gap-1">
        <h1 className="flex items-center gap-2 text-2xl font-semibold tracking-tight">
          <Swords className="size-6 text-primary" />
          Attack an agent
        </h1>
        <p className="text-sm text-muted-foreground">
          Dispatch an independent red-team at one agent endpoint. Jobs run server-side and persist,
          so you can leave and come back to the results.
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
              onChange={(e) => {
                setTargetUrl(e.target.value);
                clearStaleRun();
              }}
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
              {REDTEAM_JOB_PROFILES.map((value) => (
                <button
                  key={value}
                  type="button"
                  aria-pressed={value === profile}
                  disabled={busy}
                  onClick={() => {
                    if (value === profile) return;
                    clearStaleRun();
                    setProfile(value);
                  }}
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
            <div className="flex items-center gap-2">
              {busy && job !== null ? (
                <Button variant="outline" onClick={() => void cancel()}>
                  <X className="size-4" />
                  Cancel
                </Button>
              ) : null}
              <Button onClick={() => void run()} disabled={busy}>
                {busy ? <Activity className="size-4 animate-pulse" /> : <Play className="size-4" />}
                {busy ? 'Attacking…' : 'Attack'}
              </Button>
            </div>
          </div>
        </CardContent>
      </Card>

      {error ? (
        <p className="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
          {error}
        </p>
      ) : null}

      {job ? <ResultSummary job={job} /> : null}

      {results.length > 0 ? (
        <AttackList results={results} expanded={expanded} onToggle={setExpanded} />
      ) : job !== null && !isTerminalStatus(job.status) ? (
        <p className="text-sm text-muted-foreground">
          Attacking {job.target}… results appear when the job finishes.
        </p>
      ) : null}

      {job?.status === 'complete' ? (
        <HardenJobCard results={results} busy={busy} onHardened={() => void run()} />
      ) : null}

      {history.length > 0 ? (
        <JobHistory jobs={history} activeId={job?.id ?? null} onSelect={loadFromHistory} />
      ) : null}
    </div>
  );
}

function ResultSummary({ job }: { job: RedteamJobSummary }) {
  const percent = landedPercent(job);
  const done = isTerminalStatus(job.status);
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
              {done ? `${percent}%` : '—'}
            </span>
            <span className="pb-2 text-sm text-muted-foreground">attacks landed</span>
          </div>
          <p className="text-sm text-muted-foreground">
            {done
              ? `${job.landed} / ${job.attacks} attacks landed${job.blocked > 0 ? ` · ${job.blocked} blocked` : ''}`
              : 'running…'}
          </p>
        </div>
        <div className="flex items-center gap-3 text-xs text-muted-foreground">
          <span>
            generator <span className="font-mono">{job.generator}</span>
          </span>
          <StatusBadge status={job.status} />
        </div>
      </CardContent>
    </Card>
  );
}

function AttackList({
  results,
  expanded,
  onToggle,
}: {
  results: readonly RedteamJobResult[];
  expanded: number | null;
  onToggle: (index: number | null) => void;
}) {
  return (
    <Card className="overflow-hidden p-0">
      <ul className="divide-y">
        {results.map((item, index) => {
          const open = expanded === index;
          const evidenceId = `attack-evidence-${index}`;
          return (
            <li key={`${item.attack}-${item.seq}`}>
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
                <OutcomeBadge outcome={item.outcome} />
                <ChevronDown
                  className={cn(
                    'size-4 text-muted-foreground transition-transform motion-reduce:transition-none',
                    open && 'rotate-180',
                  )}
                />
              </button>
              <div id={evidenceId} hidden={!open} className="grid gap-3 bg-muted/40 px-4 pb-4">
                {item.prompt ? <Evidence title="Adversarial prompt" body={item.prompt} /> : null}
                <Evidence title="Agent reply" body={item.reply} traceId={item.trace_id ?? null} />
              </div>
            </li>
          );
        })}
      </ul>
    </Card>
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
    <Card className="overflow-hidden p-0">
      <CardHeader className="px-4 pt-4 pb-2">
        <CardTitle className="flex items-center gap-2 text-sm font-medium">
          <Clock className="size-4 text-muted-foreground" />
          Recent jobs
        </CardTitle>
      </CardHeader>
      <ul className="divide-y">
        {jobs.map((item) => (
          <li key={item.id}>
            <button
              type="button"
              onClick={() => onSelect(item.id)}
              aria-current={item.id === activeId}
              className={cn(
                'grid w-full grid-cols-[1fr_auto_auto] items-center gap-3 px-4 py-2.5 text-left hover:bg-accent',
                item.id === activeId && 'bg-accent',
              )}
            >
              <span className="truncate font-mono text-xs text-muted-foreground">
                {item.target}
              </span>
              <span className="text-xs tabular-nums text-muted-foreground">
                {isTerminalStatus(item.status) ? `${item.landed}/${item.attacks}` : '—'}
              </span>
              <StatusBadge status={item.status} />
            </button>
          </li>
        ))}
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

function OutcomeBadge({ outcome }: { outcome: string }) {
  if (outcome === 'landed') {
    return (
      <Badge variant="destructive" className="gap-1">
        <ShieldAlert className="size-3" />
        landed
      </Badge>
    );
  }
  if (outcome === 'error') {
    return <Badge variant="outline">error</Badge>;
  }
  return (
    <Badge variant="outline" className="gap-1 border-emerald-500/50 text-emerald-600">
      <ShieldCheck className="size-3" />
      {outcome === 'clean' ? 'safe' : outcome}
    </Badge>
  );
}

const STATUS_TONE: Record<JobStatus, string> = {
  queued: 'text-muted-foreground',
  running: 'text-amber-600',
  complete: 'text-emerald-600',
  error: 'text-destructive',
  cancelled: 'text-muted-foreground',
};

function StatusBadge({ status }: { status: JobStatus }) {
  return (
    <Badge variant="outline" className={cn('font-mono text-[11px] uppercase', STATUS_TONE[status])}>
      {status}
    </Badge>
  );
}
