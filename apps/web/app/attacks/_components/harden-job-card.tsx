'use client';

import { ChevronDown, Loader2, ShieldCheck, Sparkles, Swords } from 'lucide-react';
import { useEffect, useMemo, useRef, useState } from 'react';

import {
  applyHardenPolicy,
  buildHardenDraftFromJob,
  hardenDraftYaml,
  suggestPolicyFromJobResults,
  type HardenDraft,
} from '@/lib/redteam-harden';
import type { RedteamJobResult } from '@/lib/redteam-jobs';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { cn } from '@/lib/utils';

type ApplyState = 'idle' | 'drafting' | 'applying';

interface HardenJobCardProps {
  results: readonly RedteamJobResult[];
  /** True while a job is dispatching or polling — disables the action. */
  busy: boolean;
  /** Re-dispatch the same target/profile after the guard is applied. */
  onHardened: () => void;
}

function messageOf(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

export function HardenJobCard({ results, busy, onHardened }: HardenJobCardProps) {
  // Scanning results for landed attacks is O(results); only recompute when they change.
  const suggestion = useMemo(() => suggestPolicyFromJobResults(results), [results]);
  const [applyState, setApplyState] = useState<ApplyState>('idle');
  const [error, setError] = useState<string | null>(null);
  const [draft, setDraft] = useState<HardenDraft | null>(null);
  const [showYaml, setShowYaml] = useState(false);

  // Abort the in-flight draft/apply and stop writing state if the card unmounts
  // mid-apply (e.g. the user navigates away after clicking Harden).
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

  // Nothing landed on the guard — no policy to suggest.
  if (suggestion === null) return null;

  const harden = async () => {
    const signal = abortRef.current?.signal;
    setError(null);
    setApplyState('drafting');
    let built: HardenDraft | null;
    try {
      built = await buildHardenDraftFromJob(results, signal);
    } catch (err) {
      if (!cancelledRef.current) {
        setApplyState('idle');
        setError(messageOf(err));
      }
      return;
    }
    if (cancelledRef.current) return;
    if (built === null) {
      setApplyState('idle');
      return;
    }
    setDraft(built);
    setApplyState('applying');
    try {
      await applyHardenPolicy(built.draft, signal);
    } catch (err) {
      if (!cancelledRef.current) {
        setApplyState('idle');
        setError(`Couldn't apply the guard: ${messageOf(err)}. Is the backend running?`);
      }
      return;
    }
    if (cancelledRef.current) return;
    setApplyState('idle');
    setShowYaml(false);
    onHardened();
  };

  const yaml = hardenDraftYaml(draft?.draft ?? suggestion.fallbackDraft);

  return (
    <Card className="border-l-4 border-l-emerald-500">
      <CardContent className="grid gap-3 pt-6">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <p className="flex items-center gap-2 text-xs font-semibold tracking-wide text-emerald-700 uppercase">
            <ShieldCheck className="size-4" />
            Suggested guard
          </p>
          {draft?.source === 'llm' ? (
            <Badge variant="outline" className="gap-1 border-primary/50 text-primary">
              <Sparkles className="size-3" /> AI-suggested
            </Badge>
          ) : null}
        </div>

        <p className="text-sm">{suggestion.summary}. This guard blocks that for every reply.</p>

        <div className="flex flex-wrap gap-1">
          {suggestion.attackNames.map((name) => (
            <Badge key={name} variant="outline" className="font-mono text-[11px] text-destructive">
              {name}
            </Badge>
          ))}
        </div>

        <div className="grid gap-2">
          <button
            type="button"
            onClick={() => setShowYaml((v) => !v)}
            aria-expanded={showYaml}
            aria-controls="harden-job-yaml"
            className="flex w-fit items-center gap-1 text-xs font-semibold text-muted-foreground uppercase hover:text-foreground"
          >
            <ChevronDown
              className={cn(
                'size-4 transition-transform motion-reduce:transition-none',
                showYaml && 'rotate-180',
              )}
            />
            {showYaml ? 'Hide YAML' : 'Show YAML'}
          </button>
          <pre
            id="harden-job-yaml"
            hidden={!showYaml}
            className="overflow-x-auto rounded-md border bg-muted/40 px-3 py-2 font-mono text-xs leading-5"
          >
            {yaml}
          </pre>
        </div>

        {error ? (
          <p className="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
            {error}
          </p>
        ) : null}

        <div className="flex items-center justify-end">
          <Button onClick={() => void harden()} disabled={busy || applyState !== 'idle'}>
            {applyState === 'idle' ? (
              <Swords className="size-4" />
            ) : (
              <Loader2 className="size-4 animate-spin motion-reduce:animate-none" />
            )}
            {applyState === 'drafting'
              ? 'Building a guard…'
              : applyState === 'applying'
                ? 'Applying…'
                : 'Harden & re-run'}
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}
