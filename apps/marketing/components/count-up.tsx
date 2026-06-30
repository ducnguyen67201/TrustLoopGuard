'use client';

import { useEffect, useRef, useState } from 'react';

interface CountUpProps {
  to: number;
  prefix?: string | undefined;
  suffix?: string | undefined;
  decimals?: number | undefined;
  durationMs?: number | undefined;
}

/**
 * Animates a number from 0 to `to` once it scrolls into view.
 * Pure IntersectionObserver + requestAnimationFrame — no dependency.
 * Honors prefers-reduced-motion by jumping straight to the final value.
 */
export function CountUp({
  to,
  prefix = '',
  suffix = '',
  decimals = 0,
  durationMs = 1400,
}: CountUpProps) {
  const ref = useRef<HTMLSpanElement>(null);
  const [value, setValue] = useState(0);

  useEffect(() => {
    const el = ref.current;
    if (!el) return;

    if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
      setValue(to);
      return;
    }

    let raf = 0;
    let startTs = 0;
    const tick = (now: number) => {
      if (!startTs) startTs = now;
      const t = Math.min(1, (now - startTs) / durationMs);
      const eased = 1 - Math.pow(1 - t, 3); // easeOutCubic
      setValue(to * eased);
      if (t < 1) raf = requestAnimationFrame(tick);
    };

    const observer = new IntersectionObserver(
      (entries) => {
        if (entries[0]?.isIntersecting) {
          raf = requestAnimationFrame(tick);
          observer.disconnect();
        }
      },
      { threshold: 0.4 },
    );
    observer.observe(el);

    return () => {
      observer.disconnect();
      cancelAnimationFrame(raf);
    };
  }, [to, durationMs]);

  const formatted = value.toLocaleString('en-US', {
    minimumFractionDigits: decimals,
    maximumFractionDigits: decimals,
  });

  return (
    <span ref={ref}>
      {prefix}
      {formatted}
      {suffix}
    </span>
  );
}
