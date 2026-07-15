'use client';

import { Loader2, ShieldCheck, Sparkles, Swords } from 'lucide-react';
import { useEffect, useRef, useState } from 'react';

import {
  hardenJob,
  type HardenCandidate,
  type HardenRejection,
  type HardenResponse,
} from '@/lib/redteam-harden';
import type { RedteamAttackSession } from '@/lib/redteam-jobs';
import { setPoliciesEnabled } from '@/lib/policies';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';

type State = 'idle' | 'hardening' | 'enabling';

interface HardenJobCardProps {
  /** The job whose landed attacks we harden against. */
  jobId: string | null;
  sessions: readonly RedteamAttackSession[];
  /** True while a job is dispatching or polling — disables the action. */
  busy: boolean;
  /** Re-dispatch the same target/profile after the guard is enabled. */
  onHardened: () => void;
}

function messageOf(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

export function HardenJobCard({ jobId, sessions, busy, onHardened }: HardenJobCardProps) {
  const landed = sessions.some((session) => session.landed && !(session.outcome === 'clean'));
  const [state, setState] = useState<State>('idle');
  const [error, setError] = useState<string | null>(null);
  const [hardenResult, setHardenResult] = useState<HardenResponse | null>(null);
  const candidates = hardenResult?.candidates ?? null;
  const rejections = hardenResult?.rejections ?? [];
  const unreachable = hardenResult?.unreachable ?? [];

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
      setHardenResult(response);
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
    const disabled = candidates.filter((candidate) => !candidate.policy.enabled);
    const signal = abortRef.current?.signal;
    setError(null);
    setState('enabling');
    if (disabled.length > 0) {
      try {
        await setPoliciesEnabled(
          disabled.map((candidate) => candidate.policy.id),
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
    }
    if (cancelledRef.current) return;
    setState('idle');
    onHardened();
  };

  return (
    <Card className="border-l-4" style={{ borderLeftColor: 'var(--color-permit)' }}>
      <CardContent className="grid gap-3 pt-6">
        <p
          className="flex items-center gap-2 text-xs font-semibold tracking-wide uppercase"
          style={{ color: 'var(--color-permit)' }}
        >
          <ShieldCheck className="size-4" />
          Suggested fix
        </p>

        {candidates === null ? (
          <p className="text-sm text-muted-foreground">
            Some prompts got through. We can build or tighten a guardrail that blocks them — and
            we&apos;ll double-check it actually stops what got past before suggesting it.
          </p>
        ) : candidates.length === 0 ? (
          <div className="grid gap-2">
            <p className="text-sm text-muted-foreground">
              We found a possible guardrail, but couldn&apos;t verify it. Configure the missing
              verifier or review a semantic rule for this attack, then run the test again.
            </p>
            {rejections.length > 0 ? (
              <div className="grid gap-1">
                {rejections.map((rejection) => (
                  <p
                    key={`${rejection.reason}-${rejection.substrate}-${rejection.evidence_seqs.join('-')}`}
                    className="text-xs text-muted-foreground"
                  >
                    {rejectionSummary(rejection)}
                  </p>
                ))}
              </div>
            ) : unreachable.length > 0 ? (
              <p className="text-xs text-muted-foreground">
                Missing coverage: {unreachable.map(coverageLabel).join(', ')}.
              </p>
            ) : null}
          </div>
        ) : (
          <div className="grid gap-2">
            {candidates.map((candidate) => (
              <div
                key={candidate.policy.id}
                className="grid gap-1.5 rounded-lg border bg-muted/30 px-3 py-2.5"
              >
                <div className="flex flex-wrap items-center justify-between gap-2">
                  <div className="grid gap-1">
                    <span className="text-sm font-medium">
                      {candidate.policy.description?.trim()
                        ? candidate.policy.description
                        : operationLabel(candidate)}
                    </span>
                    <span className="text-xs text-muted-foreground">
                      {operationLabel(candidate)}
                    </span>
                  </div>
                  <div className="flex flex-wrap items-center gap-1.5">
                    <Badge variant="outline" className="gap-1 font-mono text-[11px]">
                      <Sparkles className="size-3" />
                      {candidate.substrate}
                    </Badge>
                    <Badge variant="outline" className="text-[11px]">
                      {candidate.operation === 'tighten' ? 'Tighten' : 'New'}
                    </Badge>
                  </div>
                </div>
                <p
                  className="font-mono text-xs tabular-nums"
                  style={{ color: 'var(--color-permit)' }}
                >
                  Stops {candidate.verify.blocked_landed}/{candidate.verify.landed_total} of what
                  got through · {candidate.verify.blocked_variants}/{candidate.verify.variant_total}{' '}
                  reworded tries · {candidate.verify.false_blocks} false alarms
                </p>
              </div>
            ))}
          </div>
        )}

        {error ? (
          <p
            role="alert"
            className="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive"
          >
            {error}
          </p>
        ) : null}

        <div className="flex items-center justify-end">
          {candidates === null ? (
            <Button onClick={() => void harden()} disabled={busy || state !== 'idle'}>
              {state === 'hardening' ? (
                <Loader2 className="size-4 animate-spin motion-reduce:animate-none" />
              ) : (
                <Swords className="size-4" />
              )}
              {state === 'hardening' ? 'Building a fix…' : 'Build a fix'}
            </Button>
          ) : candidates.length === 0 ? (
            <Button asChild>
              <a href={newPolicyHref(sessions)}>
                <ShieldCheck className="size-4" />
                Create rule
              </a>
            </Button>
          ) : (
            <Button onClick={() => void enableAndRerun()} disabled={busy || state !== 'idle'}>
              {state === 'enabling' ? (
                <Loader2 className="size-4 animate-spin motion-reduce:animate-none" />
              ) : (
                <ShieldCheck className="size-4" />
              )}
              {state === 'enabling'
                ? 'Turning on…'
                : candidates.some((candidate) => !candidate.policy.enabled)
                  ? 'Turn on & test again'
                  : 'Test again'}
            </Button>
          )}
        </div>
      </CardContent>
    </Card>
  );
}

function operationLabel(candidate: HardenCandidate): string {
  return candidate.operation === 'tighten'
    ? 'Tightens existing guardrail'
    : 'Creates a new guardrail';
}

function rejectionSummary(rejection: HardenRejection): string {
  const reason = rejection.message.trim() || rejection.reason.replaceAll('_', ' ');
  const evidence =
    rejection.evidence_seqs.length > 0 ? ` Evidence: #${rejection.evidence_seqs.join(', #')}.` : '';
  const verify = rejection.verify
    ? ` Checked: ${rejection.verify.blocked_landed}/${rejection.verify.landed_total} landed, ${rejection.verify.blocked_variants}/${rejection.verify.variant_total} variants, ${rejection.verify.false_blocks}/${rejection.verify.control_total} benign controls blocked.`
    : '';
  return `${reason}.${verify}${evidence}`;
}

function coverageLabel(value: string): string {
  return value.replaceAll('_', ' ');
}

function newPolicyHref(sessions: readonly RedteamAttackSession[]): string {
  if (typeof window === 'undefined') return '/policies/new';
  const current = new URLSearchParams(window.location.search);
  const next = new URLSearchParams();
  for (const key of ['workspace', 'environment']) {
    const value = current.get(key);
    if (value !== null && value.trim() !== '') next.set(key, value);
  }
  const draft = draftPolicyFromSessions(sessions);
  next.set('policyKey', draft.policyKey);
  next.set('description', draft.description);
  next.set('sourceYaml', draft.sourceYaml);
  next.set('severity', draft.severity);
  next.set('action', 'deny');
  next.set('enabled', 'true');
  const query = next.toString();
  return query === '' ? '/policies/new' : `/policies/new?${query}`;
}

function draftPolicyFromSessions(sessions: readonly RedteamAttackSession[]): {
  policyKey: string;
  description: string;
  sourceYaml: string;
  severity: 'high';
} {
  const landedText = sessions
    .filter((session) => session.landed && session.outcome !== 'clean')
    .flatMap((session) => [
      session.goal,
      ...session.events
        .filter((event) => event.kind === 'target_reply')
        .map((event) => event.content_text ?? ''),
    ])
    .join('\n')
    .toLowerCase();

  if (landedText.includes('refund')) {
    const description =
      'Blocks refund replies that proceed before identity and transaction verification.';
    const semantic =
      "The assistant offers, confirms, processes, or asks for refund destination/payment details before verifying the user's identity and transaction details. Do not match ordinary requests for transaction ID, dispute reason, or non-sensitive verification details.";
    return {
      policyKey: 'refund-verification-required',
      description,
      severity: 'high',
      sourceYaml: `id: refund-verification-required
description: ${JSON.stringify(description)}
match:
  semantic: ${JSON.stringify(semantic)}
action: deny
severity: high
`,
    };
  }

  const description = 'Blocks replies that match the landed attack pattern.';
  return {
    policyKey: 'landed-attack-output',
    description,
    severity: 'high',
    sourceYaml: `id: landed-attack-output
description: ${JSON.stringify(description)}
match:
  semantic: ${JSON.stringify(description)}
action: deny
severity: high
`,
  };
}
