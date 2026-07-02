'use client';

import { IconArrowRight } from '@tabler/icons-react';
import Link from 'next/link';
import { useEffect, useRef, useState } from 'react';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { onboardingContextQuery } from '@/components/onboarding/ConnectAgentStep';
import { http } from '@/lib/http';
import { traceListSchema, type FirstTrace } from '@/lib/onboarding';

const POLL_INTERVAL_MS = 4000;
const QUIET_FAILURES_BEFORE_NOTE = 3;

const VERDICT_VARIANTS = ['allow', 'rewrite', 'block', 'escalate'] as const;
type VerdictVariant = (typeof VERDICT_VARIANTS)[number];

function verdictVariant(decision: string): VerdictVariant | 'secondary' {
  return (VERDICT_VARIANTS as readonly string[]).includes(decision)
    ? (decision as VerdictVariant)
    : 'secondary';
}

/**
 * Onboarding step 3: a quiet terminal-style panel that polls for the
 * workspace's first trace and flips to the success state when it lands.
 * Polling errors stay silent (still listening) until a few in a row, then a
 * muted note appears — the panel never crashes out of the waiting state.
 */
export function VerifyStep({
  workspaceSlug,
  requestedEnvironmentId,
}: {
  workspaceSlug: string;
  requestedEnvironmentId?: string | undefined;
}) {
  const [trace, setTrace] = useState<FirstTrace | null>(null);
  const [failures, setFailures] = useState(0);
  const done = trace !== null;
  const doneRef = useRef(done);
  doneRef.current = done;

  useEffect(() => {
    let cancelled = false;

    async function poll() {
      if (cancelled || doneRef.current) return;
      try {
        // http.get carries ?workspace=&environment= from the page URL.
        const data = await http.get('/api/traces?limit=1', traceListSchema, {
          cache: 'no-store',
        });
        if (cancelled) return;
        setFailures(0);
        const first = data.traces[0];
        if (first !== undefined) setTrace(first);
      } catch {
        if (!cancelled) setFailures((count) => count + 1);
      }
    }

    void poll();
    const interval = setInterval(() => void poll(), POLL_INTERVAL_MS);
    return () => {
      cancelled = true;
      clearInterval(interval);
    };
  }, []);

  const contextQuery = onboardingContextQuery(workspaceSlug, requestedEnvironmentId);

  return (
    <div className="grid gap-6">
      <div className="min-w-0 rounded-lg border bg-muted/40 p-4 font-mono text-xs leading-relaxed">
        {done ? (
          <div className="grid gap-2" role="status">
            <p className="text-muted-foreground">&gt; connection established</p>
            <p className="flex flex-wrap items-center gap-2 text-foreground">
              &gt; first decision received
              <Badge variant={verdictVariant(trace.decision)}>{trace.decision}</Badge>
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
