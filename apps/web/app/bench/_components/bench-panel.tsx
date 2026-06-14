'use client';

import {
  Activity,
  BarChart3,
  ChevronDown,
  Clock,
  Play,
  Scale,
  ShieldCheck,
  Swords,
  X,
} from 'lucide-react';
import { useCallback, useEffect, useRef, useState, type ReactNode } from 'react';

import {
  bench,
  type BenchComparedCase,
  type BenchReportPayload,
  type BenchRunArmSummary,
  type BenchRunDetail,
  type BenchRunStatus,
  type BenchRunSummary,
} from '@/lib/bench-jobs';
import {
  REDTEAM_JOB_PROFILES,
  type RedteamGenerator,
  type RedteamJobProfile,
} from '@/lib/redteam-jobs';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { cn } from '@/lib/utils';

const DEFAULT_RAW_TARGET = 'http://127.0.0.1:9101';
const DEFAULT_GUARDED_TARGET = 'http://127.0.0.1:9102';
const POLL_INTERVAL_MS = 1200;
const HISTORY_LIMIT = 10;
const GENERATORS: RedteamGenerator[] = ['deterministic', 'hackagent'];

const PROFILE_COPY: Record<RedteamJobProfile, string> = {
  fast: 'Short run for local smoke checks',
  full: 'Every attack family in the runner',
  max: 'Attack and phrasing sweep',
};

const GENERATOR_COPY: Record<RedteamGenerator, string> = {
  deterministic: 'Seeded deterministic attacks',
  hackagent: 'Live generator when configured',
};

function delay(ms: number) {
  return new Promise<void>((resolve) => setTimeout(resolve, ms));
}

function messageOf(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

function isTerminalBenchStatus(status: BenchRunStatus): boolean {
  return status === 'complete' || status === 'error' || status === 'cancelled';
}

function formatPercent(value: number): string {
  return `${Math.round(value * 100)}%`;
}

function shortId(id: string): string {
  return id.length > 18 ? `${id.slice(0, 10)}...${id.slice(-6)}` : id;
}

export function BenchPanel() {
  const [rawTargetUrl, setRawTargetUrl] = useState(DEFAULT_RAW_TARGET);
  const [guardedTargetUrl, setGuardedTargetUrl] = useState(DEFAULT_GUARDED_TARGET);
  const [profile, setProfile] = useState<RedteamJobProfile>('fast');
  const [generator, setGenerator] = useState<RedteamGenerator>('deterministic');
  const [detail, setDetail] = useState<BenchRunDetail | null>(null);
  const [report, setReport] = useState<BenchReportPayload | null>(null);
  const [history, setHistory] = useState<BenchRunSummary[]>([]);
  const [expandedCase, setExpandedCase] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [dispatching, setDispatching] = useState(false);

  const activeRunRef = useRef<string | null>(null);

  useEffect(
    () => () => {
      activeRunRef.current = null;
    },
    [],
  );

  const refreshHistory = useCallback(async () => {
    try {
      setHistory(await bench.listRuns({ limit: HISTORY_LIMIT }));
    } catch {
      // History is useful but non-critical; active run errors are surfaced separately.
    }
  }, []);

  useEffect(() => {
    void refreshHistory();
  }, [refreshHistory]);

  const loadReport = useCallback(async (id: string) => {
    setReport(await bench.getReport(id));
  }, []);

  const poll = useCallback(
    async (id: string) => {
      while (activeRunRef.current === id) {
        let nextDetail: BenchRunDetail;
        try {
          nextDetail = await bench.getRun(id);
        } catch (err) {
          setError(messageOf(err));
          activeRunRef.current = null;
          return;
        }

        setDetail(nextDetail);
        if (isTerminalBenchStatus(nextDetail.run.status)) {
          activeRunRef.current = null;
          if (nextDetail.run.status === 'complete') {
            try {
              await loadReport(nextDetail.run.id);
            } catch (err) {
              setError(messageOf(err));
            }
          } else if (nextDetail.run.status === 'error') {
            setError(nextDetail.run.error ?? 'the benchmark run failed');
          }
          void refreshHistory();
          return;
        }
        await delay(POLL_INTERVAL_MS);
      }
    },
    [loadReport, refreshHistory],
  );

  const clearStaleRun = () => {
    if (detail === null && report === null && error === null) return;
    activeRunRef.current = null;
    setDetail(null);
    setReport(null);
    setError(null);
    setExpandedCase(null);
  };

  const run = useCallback(async () => {
    const raw = rawTargetUrl.trim();
    const guarded = guardedTargetUrl.trim();
    if (raw === '' || guarded === '') {
      setError('Enter both benchmark target URLs first.');
      return;
    }

    activeRunRef.current = null;
    setDispatching(true);
    setError(null);
    setDetail(null);
    setReport(null);
    setExpandedCase(null);

    let created: BenchRunDetail;
    try {
      created = await bench.createRun({
        rawTargetUrl: raw,
        guardedTargetUrl: guarded,
        profile,
        generator,
      });
    } catch (err) {
      setDispatching(false);
      setError(messageOf(err));
      return;
    }

    setDetail(created);
    setDispatching(false);
    activeRunRef.current = created.run.id;

    if (isTerminalBenchStatus(created.run.status)) {
      activeRunRef.current = null;
      if (created.run.status === 'complete') {
        try {
          await loadReport(created.run.id);
        } catch (err) {
          setError(messageOf(err));
        }
      }
      void refreshHistory();
      return;
    }
    await poll(created.run.id);
  }, [generator, guardedTargetUrl, loadReport, poll, profile, rawTargetUrl, refreshHistory]);

  const cancel = useCallback(async () => {
    if (detail === null) return;
    activeRunRef.current = null;
    try {
      const cancelled = await bench.cancel(detail.run.id);
      setDetail({ ...detail, run: cancelled });
      setReport(null);
      void refreshHistory();
    } catch (err) {
      setError(messageOf(err));
    }
  }, [detail, refreshHistory]);

  const loadFromHistory = useCallback(
    async (id: string) => {
      activeRunRef.current = null;
      setError(null);
      setReport(null);
      setExpandedCase(null);
      let nextDetail: BenchRunDetail;
      try {
        nextDetail = await bench.getRun(id);
      } catch (err) {
        setError(messageOf(err));
        return;
      }

      setDetail(nextDetail);
      if (nextDetail.run.status === 'complete') {
        try {
          await loadReport(nextDetail.run.id);
        } catch (err) {
          setError(messageOf(err));
        }
        return;
      }
      if (nextDetail.run.status === 'error') {
        setError(nextDetail.run.error ?? 'the benchmark run failed');
        return;
      }
      if (!isTerminalBenchStatus(nextDetail.run.status)) {
        activeRunRef.current = nextDetail.run.id;
        await poll(nextDetail.run.id);
      }
    },
    [loadReport, poll],
  );

  const busy = dispatching || (detail !== null && !isTerminalBenchStatus(detail.run.status));
  const hasDetail = detail !== null || report !== null || error !== null;

  return (
    <div className="grid w-full gap-6 p-4 lg:p-6">
      <header className="grid gap-1">
        <h1 className="flex items-center gap-2 text-2xl font-semibold tracking-tight">
          <Scale className="size-6 text-primary" aria-hidden="true" />
          Benchmark protection delta
        </h1>
        <p className="max-w-3xl text-sm text-muted-foreground">
          Run the same attack set against raw and guarded endpoints. Rust owns the parent run, child
          red-team jobs, and report math; this page dispatches and renders the result.
        </p>
      </header>

      <div className="grid gap-6 lg:grid-cols-[minmax(320px,380px)_1fr] lg:items-start">
        <div className="grid gap-6">
          <RunConfigCard
            rawTargetUrl={rawTargetUrl}
            guardedTargetUrl={guardedTargetUrl}
            profile={profile}
            generator={generator}
            busy={busy}
            canCancel={busy && detail !== null}
            onRawTargetChange={(value) => {
              setRawTargetUrl(value);
              clearStaleRun();
            }}
            onGuardedTargetChange={(value) => {
              setGuardedTargetUrl(value);
              clearStaleRun();
            }}
            onSelectProfile={(value) => {
              if (value === profile) return;
              setProfile(value);
              clearStaleRun();
            }}
            onSelectGenerator={(value) => {
              if (value === generator) return;
              setGenerator(value);
              clearStaleRun();
            }}
            onRun={() => void run()}
            onCancel={() => void cancel()}
          />

          {history.length > 0 ? (
            <RunHistory runs={history} activeId={detail?.run.id ?? null} onSelect={loadFromHistory} />
          ) : null}
        </div>

        <div className="grid content-start gap-6">
          {error ? (
            <p className="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
              {error}
            </p>
          ) : null}

          {detail ? <RunStatusCard detail={detail} /> : null}
          {report ? <MetricsGrid report={report} /> : null}
          {detail ? <ArmsCard detail={detail} /> : null}
          {report && report.cases.length > 0 ? (
            <CaseComparison cases={report.cases} expanded={expandedCase} onToggle={setExpandedCase} />
          ) : null}

          {hasDetail ? null : dispatching ? (
            <p className="text-sm text-muted-foreground">Dispatching benchmark...</p>
          ) : (
            <EmptyState />
          )}
        </div>
      </div>
    </div>
  );
}

function RunConfigCard({
  rawTargetUrl,
  guardedTargetUrl,
  profile,
  generator,
  busy,
  canCancel,
  onRawTargetChange,
  onGuardedTargetChange,
  onSelectProfile,
  onSelectGenerator,
  onRun,
  onCancel,
}: {
  rawTargetUrl: string;
  guardedTargetUrl: string;
  profile: RedteamJobProfile;
  generator: RedteamGenerator;
  busy: boolean;
  canCancel: boolean;
  onRawTargetChange: (value: string) => void;
  onGuardedTargetChange: (value: string) => void;
  onSelectProfile: (value: RedteamJobProfile) => void;
  onSelectGenerator: (value: RedteamGenerator) => void;
  onRun: () => void;
  onCancel: () => void;
}) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>Run setup</CardTitle>
        <CardDescription>Targets must be loopback agents exposed through the adapter.</CardDescription>
      </CardHeader>
      <CardContent className="grid gap-4">
        <div className="grid gap-2">
          <Label htmlFor="raw-target-url">Raw target URL</Label>
          <Input
            id="raw-target-url"
            value={rawTargetUrl}
            onChange={(e) => onRawTargetChange(e.target.value)}
            placeholder={DEFAULT_RAW_TARGET}
            className="font-mono"
            disabled={busy}
          />
        </div>
        <div className="grid gap-2">
          <Label htmlFor="guarded-target-url">Guarded target URL</Label>
          <Input
            id="guarded-target-url"
            value={guardedTargetUrl}
            onChange={(e) => onGuardedTargetChange(e.target.value)}
            placeholder={DEFAULT_GUARDED_TARGET}
            className="font-mono"
            disabled={busy}
          />
        </div>

        <SegmentedChoices
          label="Profile"
          values={REDTEAM_JOB_PROFILES}
          selected={profile}
          busy={busy}
          onSelect={onSelectProfile}
        />
        <p className="text-xs text-muted-foreground">{PROFILE_COPY[profile]}</p>

        <SegmentedChoices
          label="Generator"
          values={GENERATORS}
          selected={generator}
          busy={busy}
          onSelect={onSelectGenerator}
        />
        <p className="text-xs text-muted-foreground">{GENERATOR_COPY[generator]}</p>

        <div className="flex items-center gap-2">
          {canCancel ? (
            <Button variant="outline" onClick={onCancel}>
              <X className="size-4" aria-hidden="true" />
              Cancel
            </Button>
          ) : null}
          <Button onClick={onRun} disabled={busy} className="flex-1">
            {busy ? (
              <Activity className="size-4 animate-pulse" aria-hidden="true" />
            ) : (
              <Play className="size-4" aria-hidden="true" />
            )}
            {busy ? 'Running...' : 'Run benchmark'}
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}

function SegmentedChoices<T extends string>({
  label,
  values,
  selected,
  busy,
  onSelect,
}: {
  label: string;
  values: readonly T[];
  selected: T;
  busy: boolean;
  onSelect: (value: T) => void;
}) {
  return (
    <div className="grid gap-2">
      <Label>{label}</Label>
      <div className="flex flex-wrap gap-2">
        {values.map((value) => (
          <button
            key={value}
            type="button"
            aria-pressed={value === selected}
            disabled={busy}
            onClick={() => onSelect(value)}
            className={cn(
              'rounded-md border px-3 py-1.5 text-xs font-semibold uppercase transition-colors disabled:opacity-60',
              value === selected
                ? 'border-primary bg-primary text-primary-foreground'
                : 'bg-background hover:bg-accent',
            )}
          >
            {value}
          </button>
        ))}
      </div>
    </div>
  );
}

function RunHistory({
  runs,
  activeId,
  onSelect,
}: {
  runs: readonly BenchRunSummary[];
  activeId: string | null;
  onSelect: (id: string) => void;
}) {
  return (
    <Card className="overflow-hidden p-0">
      <CardHeader className="px-4 pt-4 pb-2">
        <CardTitle className="flex items-center gap-2 text-sm font-medium">
          <Clock className="size-4 text-muted-foreground" aria-hidden="true" />
          Recent benchmark runs
        </CardTitle>
      </CardHeader>
      <ul className="divide-y">
        {runs.map((run) => (
          <li key={run.id}>
            <button
              type="button"
              aria-current={run.id === activeId}
              onClick={() => onSelect(run.id)}
              className={cn(
                'grid w-full grid-cols-[1fr_auto] items-center gap-3 px-4 py-2.5 text-left hover:bg-accent',
                run.id === activeId && 'bg-accent',
              )}
            >
              <span className="grid min-w-0">
                <span className="truncate font-mono text-xs">{run.id}</span>
                <span className="text-xs text-muted-foreground">
                  {run.profile} / {run.generator}
                </span>
              </span>
              <StatusBadge status={run.status} />
            </button>
          </li>
        ))}
      </ul>
    </Card>
  );
}

function RunStatusCard({ detail }: { detail: BenchRunDetail }) {
  return (
    <Card>
      <CardContent className="flex flex-wrap items-center justify-between gap-4 pt-6">
        <div className="grid gap-1">
          <p className="text-sm font-medium">Run {shortId(detail.run.id)}</p>
          <p className="text-sm text-muted-foreground">
            {detail.run.profile} profile / {detail.run.generator} generator
          </p>
        </div>
        <StatusBadge status={detail.run.status} />
      </CardContent>
    </Card>
  );
}

function MetricsGrid({ report }: { report: BenchReportPayload }) {
  return (
    <section className="grid gap-3 md:grid-cols-2 xl:grid-cols-4">
      <MetricCard
        icon={<Swords className="size-4" aria-hidden="true" />}
        label="Raw ASR"
        value={`${formatPercent(report.raw.attack_success_rate)} raw`}
        detail={`${report.raw.landed} / ${report.raw.attacks} attacks landed`}
      />
      <MetricCard
        icon={<ShieldCheck className="size-4" aria-hidden="true" />}
        label="Guarded ASR"
        value={`${formatPercent(report.guarded.attack_success_rate)} guarded`}
        detail={`${report.guarded.blocked} blocked / ${report.guarded.attacks} attacks`}
      />
      <MetricCard
        icon={<Scale className="size-4" aria-hidden="true" />}
        label="ASR reduction"
        value={formatPercent(report.delta.attack_success_rate_reduction)}
        detail="Raw minus guarded attack success"
      />
      <MetricCard
        icon={<BarChart3 className="size-4" aria-hidden="true" />}
        label="Utility under attack"
        value={`${formatPercent(report.guarded.utility_under_attack_rate)} UA`}
        detail={`BU ${formatPercent(report.guarded.benign_utility_rate)} / false blocks ${formatPercent(
          report.guarded.false_block_rate,
        )}`}
      />
    </section>
  );
}

function MetricCard({
  icon,
  label,
  value,
  detail,
}: {
  icon: ReactNode;
  label: string;
  value: string;
  detail: string;
}) {
  return (
    <Card>
      <CardContent className="grid gap-3 p-4">
        <div className="flex items-center justify-between gap-3 text-sm text-muted-foreground">
          <span>{label}</span>
          {icon}
        </div>
        <div className="text-3xl font-semibold tabular-nums">{value}</div>
        <p className="text-xs text-muted-foreground">{detail}</p>
      </CardContent>
    </Card>
  );
}

function ArmsCard({ detail }: { detail: BenchRunDetail }) {
  return (
    <Card className="overflow-hidden p-0">
      <CardHeader className="px-4 pt-4 pb-2">
        <CardTitle className="text-sm font-medium">Benchmark arms</CardTitle>
      </CardHeader>
      <div className="grid divide-y">
        {detail.arms.map((arm) => (
          <ArmRow key={arm.arm} arm={arm} />
        ))}
      </div>
    </Card>
  );
}

function ArmRow({ arm }: { arm: BenchRunArmSummary }) {
  return (
    <div className="grid gap-2 px-4 py-3 md:grid-cols-[7rem_1fr_auto] md:items-center">
      <Badge variant="outline" className="w-fit rounded-sm">
        {arm.arm}
      </Badge>
      <div className="grid min-w-0 gap-1">
        <span className="truncate font-mono text-xs">{arm.target}</span>
        <span className="text-xs text-muted-foreground">
          checker {arm.checker_config ?? 'default'}
        </span>
      </div>
      <span className="font-mono text-xs text-muted-foreground">
        {arm.redteam_job_id ?? 'not queued'}
      </span>
    </div>
  );
}

function CaseComparison({
  cases,
  expanded,
  onToggle,
}: {
  cases: readonly BenchComparedCase[];
  expanded: number | null;
  onToggle: (index: number | null) => void;
}) {
  return (
    <Card className="overflow-hidden p-0">
      <CardHeader className="px-4 pt-4 pb-2">
        <CardTitle className="text-sm font-medium">Attack comparison</CardTitle>
      </CardHeader>
      <ul className="divide-y">
        {cases.map((item, index) => {
          const open = expanded === index;
          const evidenceId = `bench-case-${index}`;
          return (
            <li key={`${item.case_id ?? index}-${item.attack}`}>
              <button
                type="button"
                aria-expanded={open}
                aria-controls={evidenceId}
                onClick={() => onToggle(open ? null : index)}
                className="grid w-full grid-cols-[1fr_auto_2rem] items-center gap-3 px-4 py-3 text-left hover:bg-accent"
              >
                <span className="grid min-w-0">
                  <span className="truncate font-mono text-sm">{item.attack}</span>
                  <span className="truncate text-xs text-muted-foreground">{item.goal}</span>
                </span>
                <CaseStatusBadge status={item.status} />
                <ChevronDown
                  aria-hidden="true"
                  className={cn(
                    'size-4 text-muted-foreground transition-transform motion-reduce:transition-none',
                    open && 'rotate-180',
                  )}
                />
              </button>
              <div id={evidenceId} hidden={!open} className="grid gap-2 bg-muted/40 px-4 pb-4">
                <div className="grid gap-2 text-xs md:grid-cols-2">
                  <Evidence label="Raw outcome" value={item.raw_outcome} />
                  <Evidence label="Guarded outcome" value={item.guarded_outcome} />
                </div>
                <p className="text-xs text-muted-foreground">
                  {item.track ?? 'untracked'} / {item.kind ?? 'unknown'} /{' '}
                  {item.case_id ?? 'fallback identity'}
                </p>
              </div>
            </li>
          );
        })}
      </ul>
    </Card>
  );
}

function Evidence({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-md border bg-background px-3 py-2">
      <p className="text-muted-foreground">{label}</p>
      <p className="font-mono text-sm">{value}</p>
    </div>
  );
}

function EmptyState() {
  return (
    <Card className="border-dashed">
      <CardContent className="flex min-h-[18rem] flex-col items-center justify-center gap-3 p-6 text-center">
        <div className="rounded-full border border-dashed bg-muted/40 p-3">
          <Scale className="size-6 text-muted-foreground" aria-hidden="true" />
        </div>
        <div className="grid gap-1">
          <p className="text-sm font-medium">No benchmark selected</p>
          <p className="max-w-xs text-sm text-muted-foreground">
            Run a benchmark or pick a recent run to inspect its raw-vs-guarded report.
          </p>
        </div>
      </CardContent>
    </Card>
  );
}

function StatusBadge({ status }: { status: BenchRunStatus }) {
  return (
    <Badge
      variant="outline"
      className={cn(
        'rounded-sm',
        status === 'complete' && 'border-emerald-200 bg-emerald-50 text-emerald-700',
        (status === 'queued' || status === 'running') &&
          'border-blue-200 bg-blue-50 text-blue-700',
        status === 'error' && 'border-destructive/30 bg-destructive/10 text-destructive',
        status === 'cancelled' && 'border-muted-foreground/30 text-muted-foreground',
      )}
    >
      {status}
    </Badge>
  );
}

function CaseStatusBadge({ status }: { status: BenchComparedCase['status'] }) {
  return (
    <Badge
      variant="outline"
      className={cn(
        'rounded-sm',
        status === 'fixed' && 'border-emerald-200 bg-emerald-50 text-emerald-700',
        status === 'regressed' && 'border-destructive/30 bg-destructive/10 text-destructive',
        status === 'still_vulnerable' && 'border-amber-200 bg-amber-50 text-amber-700',
        status === 'unchanged' && 'border-muted-foreground/30 text-muted-foreground',
      )}
    >
      {status.replaceAll('_', ' ')}
    </Badge>
  );
}
