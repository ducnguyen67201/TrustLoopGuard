type Verdict = 'allow' | 'rewrite' | 'block' | 'escalate';

interface VerdictCardProps {
  verdict: Verdict;
  label: string;
  sample: string;
  latency: string;
}

const TOKEN: Record<Verdict, string> = {
  allow: 'var(--color-allow)',
  rewrite: 'var(--color-rewrite)',
  block: 'var(--color-block)',
  escalate: 'var(--color-escalate)',
};

export function VerdictCard({
  verdict,
  label,
  sample,
  latency,
}: VerdictCardProps) {
  const color = TOKEN[verdict];
  return (
    <article className="group glass relative overflow-hidden rounded-2xl p-5 transition-transform hover:-translate-y-0.5">
      <header className="flex items-center justify-between">
        <span
          className="inline-flex items-center gap-2 text-xs font-medium uppercase tracking-[0.16em]"
          style={{ color }}
        >
          <span
            aria-hidden
            className="inline-block h-1.5 w-1.5 rounded-full"
            style={{ background: color }}
          />
          {label}
        </span>
        <span className="font-mono text-[11px] text-[var(--color-ink-mute)]">
          {latency}
        </span>
      </header>
      <p className="mt-6 text-sm leading-relaxed text-[var(--color-ink-dim)]">
        {sample}
      </p>
      <footer className="mt-6 flex items-center gap-2 font-mono text-[11px] text-[var(--color-ink-mute)]">
        <span>verdict</span>
        <span className="text-[var(--color-ink-dim)]">·</span>
        <span style={{ color }}>{verdict}</span>
      </footer>
    </article>
  );
}
