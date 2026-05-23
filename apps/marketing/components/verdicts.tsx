import { Eyebrow } from './how';

const ROWS = [
  {
    verdict: 'allow',
    color: 'var(--color-allow)',
    summary: 'Safe to continue.',
    example: 'Return the response unchanged.',
  },
  {
    verdict: 'rewrite',
    color: 'var(--color-rewrite)',
    summary: 'Patch the unsafe parts.',
    example: 'Mask secrets, remove private data, or return a safer answer.',
  },
  {
    verdict: 'block',
    color: 'var(--color-block)',
    summary: 'Stop the action.',
    example: 'Refuse with a reason your app can show or log.',
  },
  {
    verdict: 'escalate',
    color: 'var(--color-escalate)',
    summary: 'Route to a human.',
    example: 'Attach the prompt, proposal, policy, and trace ID.',
  },
] as const;

export function Verdicts() {
  return (
    <section
      id="verdicts"
      aria-labelledby="verdicts-heading"
      className="section border-y border-[var(--color-line)]"
    >
      <div className="section-grid">
        <div>
          <Eyebrow>03. Verdicts</Eyebrow>
          <h2 id="verdicts-heading" className="section-title">
            Four outcomes your app can handle.
          </h2>
        </div>
        <div className="divide-y divide-[var(--color-line)] border-y border-[var(--color-line)]">
          {ROWS.map((row) => (
            <article key={row.verdict} className="grid gap-4 py-5 md:grid-cols-[9rem_1fr]">
              <div className="flex items-center gap-3">
                <span
                  aria-hidden
                  className="h-2.5 w-2.5 rounded-full"
                  style={{ background: row.color }}
                />
                <span className="font-mono text-sm" style={{ color: row.color }}>
                  {row.verdict}
                </span>
              </div>
              <div>
                <h3 className="text-lg font-semibold">{row.summary}</h3>
                <p className="mt-1 text-sm leading-6 text-[var(--color-muted)]">{row.example}</p>
              </div>
            </article>
          ))}
        </div>
      </div>
    </section>
  );
}
