export function Cta() {
  return (
    <section
      aria-labelledby="cta-heading"
      className="relative mx-auto max-w-6xl px-6 py-32"
    >
      <div className="glass relative overflow-hidden rounded-3xl px-8 py-20 sm:px-20">
        <div
          aria-hidden
          className="absolute -top-40 left-1/2 h-96 w-[70%] -translate-x-1/2 rounded-full blur-3xl"
          style={{
            background:
              'radial-gradient(50% 50% at 50% 50%, oklch(0.78 0.12 255 / 0.7), transparent 70%)',
          }}
        />
        <div className="relative max-w-2xl">
          <h2
            id="cta-heading"
            className="text-balance font-semibold leading-[1.02] tracking-[-0.035em]"
            style={{ fontSize: 'var(--text-display)' }}
          >
            Up and running in five minutes.{' '}
            <span className="text-[var(--color-ink-dim)]">
              One command runs the entire quickstart end-to-end.
            </span>
          </h2>
          <pre className="mt-10 rounded-2xl border border-[var(--color-hairline)] bg-white/70 px-5 py-4 font-mono text-sm backdrop-blur">
            <span className="text-[var(--color-ink-mute)]">$ </span>
            <span className="text-[var(--color-accent-deep)]">make</span>
            <span> quickstart</span>
          </pre>
          <div className="mt-8 flex flex-wrap items-center gap-3">
            <a
              href="https://github.com/ducnguyen67201/TrustLoopGuard"
              className="inline-flex items-center gap-2 rounded-full bg-[var(--color-ink)] px-6 py-3 text-sm font-medium text-white hover:bg-[var(--color-accent)] transition-colors"
            >
              Clone TrustLoopGuard
            </a>
            <a
              href="/docs"
              className="inline-flex items-center gap-2 rounded-full glass-tight px-6 py-3 text-sm font-medium text-[var(--color-ink)] hover:bg-white/80 transition-colors"
            >
              Read the docs
            </a>
          </div>
        </div>
      </div>
    </section>
  );
}
