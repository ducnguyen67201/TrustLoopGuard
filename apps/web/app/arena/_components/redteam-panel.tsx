'use client';

import { Activity, ChevronDown, Play, ShieldAlert, ShieldCheck, Swords } from 'lucide-react';
import { useCallback, useEffect, useRef, useState } from 'react';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  pollRedteamRun,
  startRedteamRun,
  successPercent,
  REDTEAM_PROFILES,
  type RedteamCase,
  type RedteamOutcome,
  type RedteamProfile,
  type RedteamReport,
  type RedteamTargetSummary,
  type RedteamTurn,
} from '@/lib/arena-redteam';
import { cn } from '@/lib/utils';

type PanelState = 'idle' | 'starting' | 'running' | 'complete' | 'error';

const POLL_INTERVAL_MS = 1200;

const PROFILE_COPY: Record<RedteamProfile, string> = {
  fast: 'Fast live · a few attacks',
  full: 'Full audit · every attack',
  max: 'Max · attack × goal sweep',
};

export function RedTeamPanel({ rawUrl, guardedUrl }: { rawUrl: string; guardedUrl: string }) {
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

  const delay = (ms: number) => new Promise<void>((resolve) => setTimeout(resolve, ms));

  const run = useCallback(async () => {
    setState('starting');
    setError(null);
    setReport(null);
    setExpanded(null);
    cancelledRef.current = false;

    let runId: string;
    try {
      const handle = await startRedteamRun({ profile, rawUrl, guardedUrl });
      runId = handle.runId;
    } catch (err) {
      setState('error');
      setError(err instanceof Error ? err.message : String(err));
      return;
    }

    setState('running');
    while (!cancelledRef.current) {
      try {
        const poll = await pollRedteamRun(runId);
        if (poll.report) setReport(poll.report);
        if (poll.status === 'complete') {
          setState('complete');
          return;
        }
        if (poll.status === 'error') {
          setState('error');
          setError(poll.report?.error ?? 'the red-team run failed');
          return;
        }
      } catch (err) {
        setState('error');
        setError(err instanceof Error ? err.message : String(err));
        return;
      }
      await delay(POLL_INTERVAL_MS);
    }
  }, [profile, rawUrl, guardedUrl]);

  const busy = state === 'starting' || state === 'running';

  return (
    <section className="grid gap-4">
      <div className="flex flex-wrap items-end justify-between gap-4 rounded-md border border-[#211f1a] bg-[#171512] p-4 text-[#f8f4e8] shadow-[4px_4px_0_#211f1a]">
        <div className="grid gap-1">
          <p className="text-xs font-semibold tracking-[0.24em] text-[#ffb36b] uppercase">
            Independent red-team
          </p>
          <h2 className="text-xl font-semibold md:text-2xl">Same agent, guarded vs not</h2>
          <p className="max-w-xl text-sm text-[#c9c0b0]">
            An independent attacker hits both agents. Watch the attack-success rate collapse once
            TrustLoopGuard is in the path.
          </p>
        </div>
        <div className="grid gap-2">
          <div className="flex flex-wrap gap-1">
            {REDTEAM_PROFILES.map((value) => (
              <button
                key={value}
                type="button"
                disabled={busy}
                onClick={() => setProfile(value)}
                className={cn(
                  'rounded-sm border px-3 py-1.5 text-xs font-semibold uppercase transition-colors disabled:opacity-60',
                  value === profile
                    ? 'border-[#ff6900] bg-[#ff6900] text-black'
                    : 'border-[#39342b] bg-[#25221d] text-[#d8cfbd] hover:bg-[#312d26]',
                )}
              >
                {value}
              </button>
            ))}
          </div>
          <div className="flex items-center justify-between gap-3">
            <span className="text-[11px] text-[#9f9585]">{PROFILE_COPY[profile]}</span>
            <Button
              onClick={() => void run()}
              disabled={busy}
              className="bg-[#ff6900] text-black hover:bg-[#ff7d24]"
            >
              {busy ? <Activity className="size-4 animate-pulse" /> : <Play className="size-4" />}
              {busy ? 'Running…' : 'Run red-team'}
            </Button>
          </div>
        </div>
      </div>

      {report ? (
        <p className="text-xs text-[#6f675b]">
          Engine: <span className="font-mono">{report.llm.mode}</span> · generator{' '}
          <span className="font-mono">{report.llm.generator}</span> · judge{' '}
          <span className="font-mono">{report.llm.judge}</span>
          {state === 'running' ? (
            <span className="ml-2">
              · {report.progress.done}/{report.progress.total} campaigns
            </span>
          ) : null}
        </p>
      ) : null}

      {error ? (
        <p className="rounded-md border border-[#d9442f] bg-[#ffe3dc] px-3 py-2 text-sm text-[#a72c1d]">
          {error}
        </p>
      ) : null}

      {report ? (
        <Scoreboard raw={report.raw} guarded={report.guarded} deltaPoints={report.deltaPoints} />
      ) : (
        <EmptyScoreboard state={state} />
      )}

      {report && report.cases.length > 0 ? (
        <AttackFeed cases={report.cases} expanded={expanded} onToggle={setExpanded} />
      ) : null}
    </section>
  );
}

function Scoreboard({
  raw,
  guarded,
  deltaPoints,
}: {
  raw: RedteamTargetSummary;
  guarded: RedteamTargetSummary;
  deltaPoints: number;
}) {
  return (
    <div className="grid items-stretch gap-3 md:grid-cols-[1fr_auto_1fr]">
      <ScoreColumn tone="danger" label="Unguarded" summary={raw} />
      <div className="flex items-center justify-center">
        <div className="grid justify-items-center gap-1 rounded-md border border-[#211f1a] bg-[#fffaf0] px-4 py-3 text-center shadow-[4px_4px_0_#211f1a]">
          <Swords className="size-5 text-[#ff6900]" />
          <div className="text-2xl font-semibold tabular-nums text-[#0f6f48]">
            ▼{Math.max(0, deltaPoints)}
          </div>
          <div className="text-[11px] text-[#6f675b] uppercase">pts blocked</div>
        </div>
      </div>
      <ScoreColumn tone="safe" label="TrustLoopGuard" summary={guarded} />
    </div>
  );
}

function ScoreColumn({
  tone,
  label,
  summary,
}: {
  tone: 'danger' | 'safe';
  label: string;
  summary: RedteamTargetSummary;
}) {
  const isSafe = tone === 'safe';
  const percent = successPercent(summary);

  return (
    <div
      className={cn(
        'grid gap-3 rounded-md border bg-[#fffaf0] p-5 shadow-[4px_4px_0_#211f1a]',
        isSafe
          ? 'border-[#13915d] border-l-4 border-l-[#13915d]'
          : 'border-[#d9442f] border-l-4 border-l-[#d9442f]',
      )}
    >
      <div className="flex items-center gap-2 text-sm font-semibold text-[#171512]">
        {isSafe ? (
          <ShieldCheck className="size-4 text-[#0f6f48]" />
        ) : (
          <ShieldAlert className="size-4 text-[#a72c1d]" />
        )}
        {label}
      </div>
      <div className="flex items-end gap-2">
        <span
          className={cn(
            'text-5xl font-semibold tabular-nums md:text-6xl',
            isSafe ? 'text-[#0f6f48]' : 'text-[#a72c1d]',
          )}
        >
          {percent}%
        </span>
        <span className="pb-2 text-xs text-[#6f675b]">attack success</span>
      </div>
      <Bar rate={summary.successRate} tone={tone} />
      <div className="text-xs text-[#6f675b]">
        {summary.landed} / {summary.attacks} attacks landed
        {summary.blocked > 0 ? ` · ${summary.blocked} blocked` : ''}
        {summary.errored > 0 ? ` · ${summary.errored} errored` : ''}
      </div>
    </div>
  );
}

function Bar({ rate, tone }: { rate: number; tone: 'danger' | 'safe' }) {
  const clamped = Math.min(1, Math.max(0, rate));
  return (
    <div className="h-3 overflow-hidden rounded-sm border border-[#211f1a] bg-[#e7ddcc]">
      <div
        className={cn(
          'h-full origin-left transition-transform duration-700 ease-out motion-reduce:transition-none',
          tone === 'safe' ? 'bg-[#13915d]' : 'bg-[#d9442f]',
        )}
        style={{ transform: `scaleX(${clamped})` }}
      />
    </div>
  );
}

function EmptyScoreboard({ state }: { state: PanelState }) {
  const message =
    state === 'starting'
      ? 'Starting the red-team backend…'
      : state === 'running'
        ? 'Attacking both agents…'
        : 'Pick a profile and run the red-team to see the before/after.';
  return (
    <div className="grid items-stretch gap-3 md:grid-cols-[1fr_auto_1fr]">
      <EmptyColumn tone="danger" label="Unguarded" />
      <div className="flex items-center justify-center">
        <Swords className="size-5 text-[#cabfa9]" />
      </div>
      <EmptyColumn tone="safe" label="TrustLoopGuard" />
      <p className="col-span-full rounded-md border border-dashed border-[#8f8576] bg-[#f8f4e8] px-4 py-6 text-center text-sm text-[#6f675b]">
        {message}
      </p>
    </div>
  );
}

function EmptyColumn({ tone, label }: { tone: 'danger' | 'safe'; label: string }) {
  const isSafe = tone === 'safe';
  return (
    <div
      className={cn(
        'grid gap-3 rounded-md border border-dashed bg-[#fffaf0] p-5',
        isSafe ? 'border-[#9ec9b5]' : 'border-[#e0a99e]',
      )}
    >
      <div className="text-sm font-semibold text-[#9f9585]">{label}</div>
      <div className="text-5xl font-semibold text-[#cabfa9] tabular-nums md:text-6xl">—</div>
      <div className="h-3 rounded-sm border border-[#d8cfbd] bg-[#efe7d8]" />
    </div>
  );
}

function AttackFeed({
  cases,
  expanded,
  onToggle,
}: {
  cases: readonly RedteamCase[];
  expanded: number | null;
  onToggle: (index: number | null) => void;
}) {
  return (
    <div className="overflow-hidden rounded-md border border-[#211f1a] bg-[#fffaf0] shadow-[4px_4px_0_#211f1a]">
      <div className="grid grid-cols-[1fr_auto_auto_2rem] items-center gap-3 border-b border-[#211f1a] bg-[#171512] px-4 py-2.5 text-[11px] font-semibold tracking-wider text-[#c9c0b0] uppercase">
        <span>Attack</span>
        <span className="w-24 text-center">Unguarded</span>
        <span className="w-24 text-center">TrustLoopGuard</span>
        <span />
      </div>
      <ul>
        {cases.map((item, index) => (
          <AttackRow
            key={`${item.attack}-${index}`}
            item={item}
            open={expanded === index}
            onToggle={() => onToggle(expanded === index ? null : index)}
          />
        ))}
      </ul>
    </div>
  );
}

function AttackRow({
  item,
  open,
  onToggle,
}: {
  item: RedteamCase;
  open: boolean;
  onToggle: () => void;
}) {
  return (
    <li className="border-b border-[#e7ddcc] last:border-b-0">
      <button
        type="button"
        onClick={onToggle}
        className="grid w-full grid-cols-[1fr_auto_auto_2rem] items-center gap-3 px-4 py-3 text-left hover:bg-[#f8f4e8]"
      >
        <span className="grid">
          <span className="font-mono text-sm text-[#171512]">{item.attack}</span>
          <span className="truncate text-xs text-[#6f675b]">{item.goal}</span>
        </span>
        <span className="flex w-24 justify-center">
          <OutcomeBadge outcome={item.raw.outcome} side="raw" />
        </span>
        <span className="flex w-24 justify-center">
          <OutcomeBadge outcome={item.guarded.outcome} side="guarded" />
        </span>
        <ChevronDown
          className={cn(
            'size-4 text-[#6f675b] transition-transform motion-reduce:transition-none',
            open && 'rotate-180',
          )}
        />
      </button>
      {open ? (
        <div className="grid gap-3 bg-[#f8f4e8] px-4 pt-1 pb-4">
          {item.prompt ? (
            <Evidence title="Adversarial prompt" body={item.prompt} />
          ) : (
            <p className="text-xs text-[#6f675b]">No prompt captured for this campaign.</p>
          )}
          <div className="grid gap-3 lg:grid-cols-2">
            <TurnEvidence label="Unguarded reply" tone="danger" turn={item.raw} />
            <TurnEvidence label="TrustLoopGuard reply" tone="safe" turn={item.guarded} />
          </div>
        </div>
      ) : null}
    </li>
  );
}

function Evidence({ title, body }: { title: string; body: string }) {
  return (
    <div className="grid gap-1">
      <div className="font-mono text-[11px] text-[#6f675b] uppercase">{title}</div>
      <div className="rounded-sm border border-[#d8cfbd] bg-[#fffaf0] px-3 py-2 text-sm leading-6 text-[#171512]">
        {body}
      </div>
    </div>
  );
}

function TurnEvidence({
  label,
  tone,
  turn,
}: {
  label: string;
  tone: 'danger' | 'safe';
  turn: RedteamTurn;
}) {
  const isSafe = tone === 'safe';
  return (
    <div
      className={cn(
        'grid gap-2 rounded-sm border bg-[#fffaf0] px-3 py-2',
        isSafe ? 'border-[#13915d]' : 'border-[#d9442f]',
      )}
    >
      <div className="flex items-center justify-between gap-2">
        <span className="font-mono text-[11px] text-[#6f675b] uppercase">{label}</span>
        <OutcomeBadge outcome={turn.outcome} side={isSafe ? 'guarded' : 'raw'} />
      </div>
      <div className="text-sm leading-6 text-[#171512]">{turn.reply || turn.detail}</div>
      {turn.traceId ? (
        <div className="font-mono text-[11px] break-all text-[#4f493f]">
          tlg.trace_id = {turn.traceId}
        </div>
      ) : null}
    </div>
  );
}

function OutcomeBadge({ outcome, side }: { outcome: RedteamOutcome; side: 'raw' | 'guarded' }) {
  const label = OUTCOME_LABEL[outcome];
  return (
    <Badge variant="outline" className={cn('rounded-sm', outcomeClassName(outcome, side))}>
      {label}
    </Badge>
  );
}

const OUTCOME_LABEL: Record<RedteamOutcome, string> = {
  landed: 'landed',
  blocked: 'blocked',
  clean: 'safe',
  error: 'error',
};

function outcomeClassName(outcome: RedteamOutcome, side: 'raw' | 'guarded'): string {
  if (outcome === 'error') return 'border-[#ff6900] bg-[#fff0df] text-[#9b4700]';
  // "landed" is bad (the attack succeeded); "blocked"/"clean" are good.
  if (outcome === 'landed') return 'border-[#d9442f] bg-[#ffe3dc] text-[#a72c1d]';
  // A clean pass on the unguarded side is neutral, not a TLG win.
  if (outcome === 'clean' && side === 'raw') return 'border-[#d8cfbd] bg-[#f8f4e8] text-[#6f675b]';
  return 'border-[#13915d] bg-[#dbf7e7] text-[#0f6f48]';
}
