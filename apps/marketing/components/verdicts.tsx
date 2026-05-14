const ROWS = [
  {
    verdict: 'allow',
    color: 'var(--color-allow)',
    summary: 'Safe to deliver as-is.',
    detail:
      'Nothing in the response crossed your policy thresholds. Every verdict carries a trace ID so you can audit any call later.',
  },
  {
    verdict: 'rewrite',
    color: 'var(--color-rewrite)',
    summary: 'Patch the unsafe parts, keep the answer.',
    detail:
      'PII is redacted, secrets are masked, and refusals are stitched in cleanly. You get back a ready-to-ship version of the response.',
  },
  {
    verdict: 'block',
    color: 'var(--color-block)',
    summary: 'Refuse, with a reason your team can act on.',
    detail:
      'The verdict tells you which checks fired, at what confidence, and which spans of text triggered them.',
  },
  {
    verdict: 'escalate',
    color: 'var(--color-escalate)',
    summary: 'Route to a human reviewer.',
    detail:
      'For ambiguous or high-stakes calls, TrustLoopGuard hands off to your review channel with the full context attached.',
  },
] as const;

export function Verdicts() {
  return (
    <section
      id="verdicts"
      aria-labelledby="verdicts-heading"
      className="border-b border-[var(--color-border)]"
    >
      <div className="mx-auto max-w-6xl px-6 py-24 sm:py-32">
        <span className="eyebrow">Four clear outcomes</span>
        <h2
          id="verdicts-heading"
          className="mt-5 max-w-3xl text-balance font-semibold leading-[1.05] tracking-[-0.025em]"
          style={{ fontSize: 'var(--text-display)' }}
        >
          Every call returns one of four unambiguous outcomes.
        </h2>

        <div className="mt-14 divide-y divide-[var(--color-border)] border-y border-[var(--color-border)]">
          {ROWS.map((row) => (
            <article
              key={row.verdict}
              className="grid grid-cols-1 gap-4 py-6 md:grid-cols-[200px_1fr_auto] md:items-center md:gap-8"
            >
              <div className="flex items-center gap-3">
                <span
                  aria-hidden
                  className="inline-block h-1.5 w-1.5 rounded-full"
                  style={{ background: row.color }}
                />
                <span
                  className="font-mono text-[11px] uppercase tracking-[0.18em]"
                  style={{ color: row.color }}
                >
                  {row.verdict}
                </span>
              </div>
              <div>
                <div className="text-base tracking-tight text-[var(--color-ink)]">
                  {row.summary}
                </div>
                <p className="mt-1.5 text-sm leading-relaxed text-[var(--color-ink-dim)] md:max-w-2xl">
                  {row.detail}
                </p>
              </div>
              <a
                href="#sdk"
                className="link-quiet inline-flex items-center gap-1 self-start font-mono text-xs md:self-center"
              >
                Handler
                <span aria-hidden>→</span>
              </a>
            </article>
          ))}
        </div>
      </div>
    </section>
  );
}
