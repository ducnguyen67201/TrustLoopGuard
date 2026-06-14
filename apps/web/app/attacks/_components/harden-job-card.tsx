'use client';

import { Loader2, ShieldCheck, Sparkles, Swords } from 'lucide-react';
import { useEffect, useRef, useState } from 'react';

import { hardenJob, type HardenCandidate } from '@/lib/redteam-harden';
import type { RedteamJobResult } from '@/lib/redteam-jobs';
import { setPoliciesEnabled } from '@/lib/policies';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';

type State = 'idle' | 'hardening' | 'enabling';

interface HardenJobCardProps {
  /** The job whose landed attacks we harden against. */
  jobId: string | null;
  results: readonly RedteamJobResult[];
  /** True while a job is dispatching or polling — disables the action. */
  busy: boolean;
  /** Re-dispatch the same target/profile after the guard is enabled. */
  onHardened: () => void;
}

function messageOf(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

export function HardenJobCard({ jobId, results, busy, onHardened }: HardenJobCardProps) {
  const landed = results.some((result) => result.landed && !(result.outcome === 'clean'));
  const [state, setState] = useState<State>('idle');
  const [error, setError] = useState<string | null>(null);
  const [candidates, setCandidates] = useState<HardenCandidate[] | null>(null);

  const cancelledRef = useRef(false);
  const abortRef = useRef<AbortController | null>(null);
  useEffect(() => {
    const controller = new AbortController();
    abortRef.current = controller;
    cancelledRef.current = false;
    return () => {
      cancelledRef.current = true;
      controller.abort();
    };
  }, []);

  // Nothing landed (or no job) — no guardrail to synthesize.
  if (!landed || jobId === null) return null;

  const harden = async () => {
    const signal = abortRef.current?.signal;
    setError(null);
    setState('hardening');
    try {
      const response = await hardenJob(jobId, true, signal);
      if (cancelledRef.current) return;
      setCandidates(response.candidates);
      setState('idle');
    } catch (err) {
      if (!cancelledRef.current) {
        setState('idle');
        setError(`Couldn't synthesize a guard: ${messageOf(err)}. Is the backend running?`);
      }
    }
  };

  const enableAndRerun = async () => {
    if (candidates === null || candidates.length === 0) return;
    const signal = abortRef.current?.signal;
    setError(null);
    setState('enabling');
    try {
      await setPoliciesEnabled(
        candidates.map((candidate) => candidate.policy.id),
        true,
        signal,
      );
    } catch (err) {
      if (!cancelledRef.current) {
        setState('idle');
        setError(`Couldn't enable the guard: ${messageOf(err)}.`);
      }
      return;
    }
    if (cancelledRef.current) return;
    setState('idle');
    onHardened();
  };

  return (
    <Card className="border-l-4 border-l-emerald-500">
      <CardContent className="grid gap-3 pt-6">
        <p className="flex items-center gap-2 text-xs font-semibold tracking-wide text-emerald-700 uppercase">
          <ShieldCheck className="size-4" />
          Suggested guard
        </p>

        {candidates === null ? (
          <p className="text-sm">
            Synthesize a guardrail from the attacks that landed. TrustLoopGuard verifies each
            candidate against what landed before recommending it.
          </p>
        ) : candidates.length === 0 ? (
          <p className="text-sm text-muted-foreground">
            No guardrail could be verified for what landed. The attack may need an action-level
            defense that this run&apos;s traces can&apos;t reach yet.
          </p>
        ) : (
          <div className="grid gap-2">
            {candidates.map((candidate) => (
              <div key={candidate.policy.id} className="grid gap-1 rounded-md border px-3 py-2">
                <div className="flex flex-wrap items-center justify-between gap-2">
                  <span className="font-mono text-xs">{candidate.policy.id}</span>
                  <Badge variant="outline" className="gap-1 text-[11px]">
                    <Sparkles className="size-3" />
                    {candidate.substrate}
                  </Badge>
                </div>
                {candidate.policy.description ? (
                  <p className="text-sm">{candidate.policy.description}</p>
                ) : null}
                <p className="text-xs text-emerald-700">
                  Blocks {candidate.verify.blocked_landed}/{candidate.verify.landed_total} landed ·{' '}
                  {candidate.verify.blocked_variants}/{candidate.verify.variant_total} variants ·{' '}
                  {candidate.verify.false_blocks} false-blocks
                </p>
              </div>
            ))}
          </div>
        )}

        {error ? (
          <p className="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
            {error}
          </p>
        ) : null}

        <div className="flex items-center justify-end">
          {candidates === null || candidates.length === 0 ? (
            <Button onClick={() => void harden()} disabled={busy || state !== 'idle'}>
              {state === 'hardening' ? (
                <Loader2 className="size-4 animate-spin motion-reduce:animate-none" />
              ) : (
                <Swords className="size-4" />
              )}
              {state === 'hardening' ? 'Synthesizing…' : 'Synthesize guard'}
            </Button>
          ) : (
            <Button onClick={() => void enableAndRerun()} disabled={busy || state !== 'idle'}>
              {state === 'enabling' ? (
                <Loader2 className="size-4 animate-spin motion-reduce:animate-none" />
              ) : (
                <ShieldCheck className="size-4" />
              )}
              {state === 'enabling' ? 'Enabling…' : 'Enable & re-run'}
            </Button>
          )}
        </div>
      </CardContent>
    </Card>
  );
}
