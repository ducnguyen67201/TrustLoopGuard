'use client';

import { IconArrowRight } from '@tabler/icons-react';
import Link from 'next/link';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { onboardingContextQuery } from '@/components/onboarding/ConnectAgentStep';
import { useFirstTrace, effectVariant } from '@/components/onboarding/useFirstTrace';

const QUIET_FAILURES_BEFORE_NOTE = 3;

/**
 * Onboarding step 3: a quiet terminal-style panel that polls for the
 * workspace's first trace (via useFirstTrace) and flips to the success state
 * when it lands. Polling errors stay silent (still listening) until a few in
 * a row, then a muted note appears — the panel never crashes out of the
 * waiting state.
 */
export function VerifyStep({
  workspaceSlug,
  requestedEnvironmentId,
}: {
  workspaceSlug: string;
  requestedEnvironmentId?: string | undefined;
}) {
  const { trace, failures } = useFirstTrace();
  const done = trace !== null;

  const contextQuery = onboardingContextQuery(workspaceSlug, requestedEnvironmentId);

  return (
    <div className="grid gap-6">
      <div className="min-w-0 rounded-lg border bg-muted/40 p-4 font-mono text-xs leading-relaxed">
        {done ? (
          <div className="grid gap-2" role="status">
            <p className="text-muted-foreground">&gt; connection established</p>
            <p className="flex flex-wrap items-center gap-2 text-foreground">
              &gt; first decision received
              <Badge variant={effectVariant(trace.decision)}>{trace.decision}</Badge>
              <span className="tabular-nums text-muted-foreground">{trace.elapsed_ms}ms</span>
            </p>
            <p className="break-all text-muted-foreground">&gt; trace {trace.trace_id}</p>
          </div>
        ) : (
          <div className="grid gap-2" role="status" aria-live="polite">
            <p className="flex items-center gap-2 text-foreground">
              <span aria-hidden className="relative flex size-1.5 shrink-0">
                <span className="absolute inline-flex size-full animate-ping rounded-full bg-primary/60 motion-reduce:hidden" />
                <span className="relative inline-flex size-1.5 rounded-full bg-primary" />
              </span>
              &gt; listening for events…
            </p>
            <p className="text-muted-foreground">
              &gt; run your agent once — its first request shows up here in seconds.
            </p>
            {failures >= QUIET_FAILURES_BEFORE_NOTE ? (
              <p className="text-muted-foreground">
                &gt; having trouble reaching the server — still retrying.
              </p>
            ) : null}
          </div>
        )}
      </div>

      {done ? (
        <div className="flex flex-wrap items-center gap-2">
          <Button asChild>
            <Link href={`/policies${contextQuery}`}>
              Create your first policy
              <IconArrowRight aria-hidden />
            </Link>
          </Button>
          <Button asChild variant="ghost">
            <Link href={`/${contextQuery}`}>Go to dashboard</Link>
          </Button>
        </div>
      ) : (
        <div>
          <Button asChild variant="ghost">
            <Link href={`/${contextQuery}`}>Skip — I&apos;ll connect later</Link>
          </Button>
        </div>
      )}
    </div>
  );
}
