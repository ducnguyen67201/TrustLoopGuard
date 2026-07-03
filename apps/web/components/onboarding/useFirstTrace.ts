'use client';

import { useEffect, useRef, useState } from 'react';

import { http } from '@/lib/http';
import { traceListSchema, type FirstTrace } from '@/lib/onboarding';

const POLL_INTERVAL_MS = 4000;

const VERDICT_VARIANTS = ['allow', 'rewrite', 'block', 'escalate'] as const;
type VerdictVariant = (typeof VERDICT_VARIANTS)[number];

export function verdictVariant(decision: string): VerdictVariant | 'secondary' {
  return (VERDICT_VARIANTS as readonly string[]).includes(decision)
    ? (decision as VerdictVariant)
    : 'secondary';
}

/**
 * Polls for the workspace's first trace every few seconds and stops once one
 * lands. Polling errors stay silent (the caller decides when `failures` is
 * worth surfacing) — the waiting state never crashes. Shared by the connect
 * step's inline status and the verify page.
 */
export function useFirstTrace(): { trace: FirstTrace | null; failures: number } {
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

  return { trace, failures };
}
