'use client';

import { useEffect, useRef } from 'react';
import { toast } from 'sonner';

const POLL_MS = 60_000;

// A newer deploy is out when the server reports a build id that differs from
// the one baked into this tab's bundle. Ignore missing/dev stamps so local
// rebuilds don't nag.
export function shouldNotify(current: string | undefined, latest: string | null): boolean {
  if (!current || !latest) return false;
  if (current.startsWith('dev-')) return false;
  return latest !== current;
}

export function VersionWatcher() {
  const current = process.env['NEXT_PUBLIC_BUILD_ID'];
  const notified = useRef(false);

  useEffect(() => {
    let cancelled = false;

    async function check() {
      if (cancelled || notified.current) return;
      try {
        const res = await fetch('/api/version', { cache: 'no-store' });
        if (!res.ok) return;
        const { buildId } = (await res.json()) as { buildId: string | null };
        if (shouldNotify(current, buildId)) {
          notified.current = true;
          toast('A new version is available', {
            description: "Refresh to get the latest.",
            duration: Infinity,
            action: { label: 'Refresh', onClick: () => window.location.reload() },
          });
        }
      } catch {
        // Network blip — try again next tick.
      }
    }

    const interval = setInterval(check, POLL_MS);
    // Catch users returning to a tab that's been idle since a deploy.
    const onVisible = () => {
      if (document.visibilityState === 'visible') check();
    };
    document.addEventListener('visibilitychange', onVisible);

    return () => {
      cancelled = true;
      clearInterval(interval);
      document.removeEventListener('visibilitychange', onVisible);
    };
  }, [current]);

  return null;
}
