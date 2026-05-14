import { GITHUB_URL } from '@/lib/github';

export function Cta() {
  return (
    <section
      aria-labelledby="cta-heading"
      className="border-b border-[var(--color-border)]"
    >
      <div className="mx-auto max-w-6xl px-6 py-24 sm:py-32">
        <div className="surface px-8 py-14 sm:px-14 sm:py-20">
          <div className="max-w-2xl">
            <span className="eyebrow">Get started</span>
            <h2
              id="cta-heading"
              className="mt-5 text-balance font-semibold leading-[1.05] tracking-[-0.025em]"
              style={{ fontSize: 'var(--text-display)' }}
            >
              Up and running in five minutes.
            </h2>
            <p className="mt-4 max-w-lg text-base leading-relaxed text-[var(--color-ink-dim)]">
              One command runs the entire quickstart end-to-end — server,
              SDKs in three languages, and a sample policy.
            </p>

            <div className="mt-8 inline-flex items-center gap-3 rounded-lg border border-[var(--color-border)] bg-[var(--color-canvas-soft)] px-4 py-3 font-mono text-sm">
              <span className="text-[var(--color-ink-mute)]">$</span>
              <span className="text-[var(--color-ink)]">make quickstart</span>
            </div>

            <div className="mt-8 flex flex-wrap items-center gap-3">
              <a href={GITHUB_URL} className="btn-primary">
                View on GitHub
              </a>
              <a href="/docs" className="btn-ghost">
                Read the docs
              </a>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}
