const REASONS = [
  {
    title: 'Real-time, not after-the-fact',
    body: 'Safety checks run inline, before output reaches your customers. No nightly eval batches, no incident reviews after the fact.',
  },
  {
    title: 'Every verdict is auditable',
    body: 'Each call returns the verdict, the reason, a trace ID, end-to-end latency, and the exact checks that fired.',
  },
  {
    title: 'Policies in version control',
    body: 'Plain YAML. Diffable, reviewable, and shipped through the same pull-request flow as the rest of your code.',
  },
  {
    title: 'Consistent across your stack',
    body: 'TypeScript, Python, and Rust SDKs all enforce the same behavior. The verdict you see in staging is the verdict you ship in production.',
  },
] as const;

export function Why() {
  return (
    <section
      id="why"
      aria-labelledby="why-heading"
      className="border-b border-[var(--color-border)]"
    >
      <div className="mx-auto max-w-6xl px-6 py-24 sm:py-32">
        <span className="eyebrow">Why TrustLoopGuard</span>
        <h2
          id="why-heading"
          className="mt-5 max-w-3xl text-balance font-semibold leading-[1.05] tracking-[-0.025em]"
          style={{ fontSize: 'var(--text-display)' }}
        >
          Guardrails that ship alongside your agent, not after the fact.
        </h2>

        <div className="mt-14 grid gap-px overflow-hidden rounded-xl border border-[var(--color-border)] bg-[var(--color-border)] sm:grid-cols-2">
          {REASONS.map((reason) => (
            <article
              key={reason.title}
              className="bg-[var(--color-surface)] p-8 transition-colors hover:bg-[var(--color-canvas-soft)]"
            >
              <h3 className="text-lg font-medium tracking-tight">
                {reason.title}
              </h3>
              <p className="mt-3 max-w-md text-sm leading-relaxed text-[var(--color-ink-dim)]">
                {reason.body}
              </p>
            </article>
          ))}
        </div>
      </div>
    </section>
  );
}
