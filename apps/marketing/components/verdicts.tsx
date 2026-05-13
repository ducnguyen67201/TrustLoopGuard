import { Eyebrow } from './how';

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
      className="relative mx-auto max-w-6xl px-6 py-32"
    >
      <Eyebrow>Four clear outcomes</Eyebrow>
      <h2
        id="verdicts-heading"
        className="mt-4 max-w-3xl text-balance font-medium leading-[1.04] tracking-[-0.03em]"
        style={{ fontSize: 'var(--text-display)' }}
      >
        Every call returns one of four{' '}
        <span className="text-[var(--color-ink-dim)]">
          unambiguous outcomes.
        </span>
      </h2>

      <div className="mt-16 divide-y divide-[var(--color-hairline)] border-y border-[var(--color-hairline)]">
        {ROWS.map((row) => (
          <article
            key={row.verdict}
            className="group grid grid-cols-1 gap-6 py-8 transition-colors hover:bg-white/40 md:grid-cols-[1.2fr_2fr_auto] md:items-center md:px-4"
          >
            <div className="flex items-center gap-4">
              <span
                aria-hidden
                className="inline-block h-2.5 w-2.5 rounded-full"
                style={{ background: row.color }}
              />
              <span
                className="font-mono text-xs uppercase tracking-[0.18em]"
                style={{ color: row.color }}
              >
                {row.verdict}
              </span>
              <span className="text-lg tracking-tight">{row.summary}</span>
            </div>
            <p className="text-sm leading-relaxed text-[var(--color-ink-dim)] md:max-w-xl">
              {row.detail}
            </p>
            <a
              href={`#sdk`}
              className="inline-flex items-center gap-1.5 self-start rounded-full glass px-3 py-1.5 text-xs text-[var(--color-ink-dim)] transition group-hover:text-[var(--color-ink)]"
            >
              Handler
              <span aria-hidden>→</span>
            </a>
          </article>
        ))}
      </div>
    </section>
  );
}
