'use client';

import { ChevronDown, Loader2, ShieldCheck, Sparkles, Swords, Trophy } from 'lucide-react';
import { useCallback, useEffect, useRef, useState } from 'react';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  applyHardenPolicy,
  buildHardenDraft,
  hardenDraftYaml,
  suggestPolicyFromReport,
  type HardenDraft,
} from '@/lib/arena-harden';
import { successPercent, type RedteamReport } from '@/lib/arena-redteam';
import { cn } from '@/lib/utils';

type ApplyState = 'idle' | 'drafting' | 'applying';

interface HardenRound {
  round: number;
  beforePct: number;
  afterPct: number;
  deltaPts: number;
  policyId: string;
}

interface PendingHarden {
  beforePct: number;
  policyId: string;
}

interface HardenPanelProps {
  report: RedteamReport | null;
  /** True while a red-team run is starting or polling. */
  busy: boolean;
  /** Identity of the run config (profile + targets). A change resets the loop;
   * the report going transiently null during an auto-rerun does NOT. */
  configKey: string;
  /** Re-run the same campaign after a policy is applied (the auto-rerun). */
  onHardened: () => void;
}

export function HardenPanel({ report, busy, configKey, onHardened }: HardenPanelProps) {
  const [rounds, setRounds] = useState<HardenRound[]>([]);
  const [applyState, setApplyState] = useState<ApplyState>('idle');
  const [error, setError] = useState<string | null>(null);
  const [showYaml, setShowYaml] = useState(false);
  const [draft, setDraft] = useState<HardenDraft | null>(null);

  // The report shown when the user clicked "Harden". The pending round is
  // finalised only when a DIFFERENT completed report arrives (the re-run),
  // so we never fill the "after" number from the pre-harden report.
  const [pending, setPending] = useState<PendingHarden | null>(null);
  const baselineReportRef = useRef<RedteamReport | null>(null);
  const cancelledRef = useRef(false);
  useEffect(
    () => () => {
      cancelledRef.current = true;
    },
    [],
  );

  // A config change (new agent / profile / targets) resets the loop. The report
  // going transiently null during an auto-rerun must NOT — so key on configKey,
  // not on the report. Skip the first render so an initial config is not a reset.
  const firstConfigRef = useRef(true);
  useEffect(() => {
    if (firstConfigRef.current) {
      firstConfigRef.current = false;
      return;
    }
    setRounds([]);
    setPending(null);
    setApplyState('idle');
    setError(null);
    setShowYaml(false);
    setDraft(null);
    baselineReportRef.current = null;
  }, [configKey]);

  // Finalise the pending round when the re-run completes.
  useEffect(() => {
    if (pending === null || report === null) return;
    if (report.status !== 'complete') return;
    if (report === baselineReportRef.current) return; // still the pre-harden report
    const afterPct = successPercent(report.guarded);
    setRounds((prev) => [
      ...prev,
      {
        round: prev.length + 1,
        beforePct: pending.beforePct,
        afterPct,
        deltaPts: Math.max(0, pending.beforePct - afterPct),
        policyId: pending.policyId,
      },
    ]);
    setPending(null);
    // Drop the applied draft so the next suggestion shows its own policy, not the
    // one we just applied.
    setDraft(null);
  }, [report, pending]);

  const suggestion = report !== null ? suggestPolicyFromReport(report) : null;
  const guardedPct = report !== null ? successPercent(report.guarded) : null;
  const isComplete = report?.status === 'complete';
  const reRunning = pending !== null && (busy || !isComplete);
  const won =
    rounds.length > 0 &&
    isComplete &&
    report !== null &&
    report.guarded.attacks > 0 &&
    report.guarded.successRate === 0;

  const harden = useCallback(async () => {
    if (report === null || guardedPct === null) return;
    setError(null);
    setApplyState('drafting');
    let built: HardenDraft | null;
    try {
      built = await buildHardenDraft(report);
    } catch (err) {
      setApplyState('idle');
      setError(err instanceof Error ? err.message : String(err));
      return;
    }
    if (built === null) {
      setApplyState('idle');
      return;
    }
    if (cancelledRef.current) return;
    setDraft(built);
    setApplyState('applying');
    let policyId: string;
    try {
      policyId = await applyHardenPolicy(built.draft);
    } catch (err) {
      setApplyState('idle');
      setError(
        `Couldn't apply the guard: ${err instanceof Error ? err.message : String(err)}. Is the backend running?`,
      );
      return;
    }
    if (cancelledRef.current) return;
    // Order matters: stamp the baseline (this pre-harden report) BEFORE onHardened()
    // kicks off the re-run, so the finalise effect only fills the round from the
    // NEXT completed report, never this one.
    baselineReportRef.current = report;
    setPending({ beforePct: guardedPct, policyId });
    setApplyState('idle');
    setShowYaml(false);
    onHardened();
  }, [report, guardedPct, onHardened]);

  // Nothing to offer until a run has finished and at least one attack landed on
  // the guard. But once a loop is underway (a round recorded, or a harden pending
  // its auto-rerun), keep rendering even while the report is transiently null.
  if (suggestion === null && rounds.length === 0 && pending === null) return null;

  return (
    <section aria-labelledby="harden-heading" className="grid gap-4">
      <h2 id="harden-heading" className="sr-only">
        Harden the guard
      </h2>
      <p aria-live="polite" className="sr-only">
        {reRunning
          ? 'Re-running the same attacks against the new guard.'
          : won
            ? 'Hardened. Nothing got through.'
            : suggestion !== null
              ? suggestion.summary
              : ''}
      </p>

      {error ? (
        <p className="rounded-md border border-[#d9442f] bg-[#ffe3dc] px-3 py-2 text-sm text-[#a72c1d]">
          {error}
        </p>
      ) : null}

      {won && report !== null ? <WinCard report={report} rounds={rounds} /> : null}

      {!won && suggestion !== null && !reRunning ? (
        <div className="grid gap-3 rounded-md border border-[#211f1a] border-l-4 border-l-[#13915d] bg-[#fffaf0] p-4 shadow-[4px_4px_0_#211f1a]">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <p className="flex items-center gap-2 text-xs font-semibold tracking-[0.24em] text-[#0f6f48] uppercase">
              <ShieldCheck className="size-4" />
              Suggested guard
            </p>
            {draft?.source === 'llm' ? (
              <Badge variant="outline" className="rounded-sm border-[#ff6900] text-[#9b4700]">
                <Sparkles className="size-3" /> AI-suggested
              </Badge>
            ) : null}
          </div>

          <p className="text-sm text-[#171512]">
            {suggestion.summary}. This guard blocks that for every reply.
          </p>

          <div className="flex flex-wrap gap-1">
            {suggestion.attackNames.map((name) => (
              <Badge
                key={name}
                variant="outline"
                className="rounded-sm border-[#d9442f] bg-[#ffe3dc] font-mono text-[11px] text-[#a72c1d]"
              >
                {name}
              </Badge>
            ))}
          </div>

          <YamlDisclosure
            yaml={hardenDraftYaml(draft?.draft ?? suggestion.fallbackDraft)}
            open={showYaml}
            onToggle={() => setShowYaml((v) => !v)}
          />

          <div className="flex items-center justify-end">
            <Button
              onClick={() => void harden()}
              disabled={busy || applyState !== 'idle'}
              className="bg-[#ff6900] text-black hover:bg-[#ff7d24]"
            >
              {applyState === 'idle' ? (
                <Swords className="size-4" />
              ) : (
                <Loader2 className="size-4 animate-spin motion-reduce:animate-none" />
              )}
              {applyState === 'drafting'
                ? 'Building a guard…'
                : applyState === 'applying'
                  ? 'Applying…'
                  : 'Harden against these'}
            </Button>
          </div>
        </div>
      ) : null}

      {rounds.length > 0 || reRunning ? (
        <RoundsTimeline rounds={rounds} reRunning={reRunning} guardedPct={guardedPct} />
      ) : null}
    </section>
  );
}

function YamlDisclosure({
  yaml,
  open,
  onToggle,
}: {
  yaml: string;
  open: boolean;
  onToggle: () => void;
}) {
  return (
    <div className="grid gap-2">
      <button
        type="button"
        onClick={onToggle}
        aria-expanded={open}
        aria-controls="harden-yaml"
        className="flex w-fit items-center gap-1 text-xs font-semibold text-[#6f675b] uppercase hover:text-[#171512]"
      >
        <ChevronDown
          className={cn(
            'size-4 transition-transform motion-reduce:transition-none',
            open && 'rotate-180',
          )}
        />
        {open ? 'Hide YAML' : 'Show YAML'}
      </button>
      {/* Always rendered so the button's aria-controls reference stays valid;
          toggled with `hidden` rather than conditional mounting. */}
      <pre
        id="harden-yaml"
        hidden={!open}
        className="overflow-x-auto rounded-sm border border-[#d8cfbd] bg-[#171512] px-3 py-2 font-mono text-[12px] leading-5 text-[#f8f4e8]"
      >
        {yaml}
      </pre>
    </div>
  );
}

function RoundsTimeline({
  rounds,
  reRunning,
  guardedPct,
}: {
  rounds: HardenRound[];
  reRunning: boolean;
  guardedPct: number | null;
}) {
  return (
    <div className="grid gap-2 rounded-md border border-[#211f1a] bg-[#fffaf0] p-4 shadow-[4px_4px_0_#211f1a]">
      <p className="text-xs font-semibold tracking-[0.24em] text-[#171512] uppercase">
        Hardening rounds
      </p>
      <ul className="grid gap-1.5">
        {rounds.map((r) => (
          <li
            key={r.round}
            className="grid grid-cols-[auto_1fr_auto] items-center gap-3 border-b border-[#e7ddcc] pb-1.5 text-sm last:border-b-0"
          >
            <span className="flex items-center gap-2 font-semibold text-[#0f6f48]">
              <ShieldCheck className="size-4" />
              Round {r.round}
            </span>
            <span className="tabular-nums text-[#171512]">
              guarded {r.beforePct}% →{' '}
              <span className="font-semibold text-[#0f6f48]">{r.afterPct}%</span>
            </span>
            <span className="flex items-center gap-2">
              <span className="tabular-nums text-[#0f6f48]">▼{r.deltaPts} pts</span>
              <span className="font-mono text-[11px] text-[#6f675b]">{r.policyId}</span>
            </span>
          </li>
        ))}
        {reRunning ? (
          <li className="grid grid-cols-[auto_1fr] items-center gap-3 text-sm text-[#6f675b]">
            <span className="flex items-center gap-2 font-semibold">
              <Loader2 className="size-4 animate-spin motion-reduce:animate-none" />
              Round {rounds.length + 1}
            </span>
            <span>applying… re-running the same attacks…</span>
          </li>
        ) : guardedPct !== null && guardedPct > 0 ? (
          <li className="pt-1 text-xs text-[#6f675b]">
            {guardedPct}% still gets through — harden again to close the rest.
          </li>
        ) : null}
      </ul>
    </div>
  );
}

function WinCard({ report, rounds }: { report: RedteamReport; rounds: HardenRound[] }) {
  const blocked = report.guarded.blocked;
  return (
    <div className="grid gap-2 rounded-md border border-[#13915d] border-l-4 border-l-[#13915d] bg-[#dbf7e7] p-5 shadow-[4px_4px_0_#211f1a]">
      <p className="flex items-center gap-2 text-lg font-semibold text-[#0f6f48]">
        <Trophy className="size-5" />
        Hardened. Nothing got through.
      </p>
      <p className="text-sm text-[#0f6f48]">
        {blocked} {blocked === 1 ? 'attack' : 'attacks'} blocked across {rounds.length}{' '}
        {rounds.length === 1 ? 'round' : 'rounds'}.
      </p>
    </div>
  );
}
