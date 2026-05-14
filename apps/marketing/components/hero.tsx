import { HeroVisual } from './hero-visual';
import { GITHUB_URL } from '@/lib/github';

export function Hero() {
  return (
    <section
      aria-labelledby="hero-heading"
      className="relative border-b border-[var(--color-border)]"
    >
      <div className="mx-auto grid max-w-6xl gap-16 px-6 py-20 sm:py-28 lg:grid-cols-[1.1fr_1fr] lg:items-center lg:py-32">
        <div>
          <div className="float-in float-in-1">
            <a
              href={GITHUB_URL}
              className="inline-flex items-center gap-2 rounded-full border border-[var(--color-border)] bg-[var(--color-surface)] px-3 py-1 text-xs text-[var(--color-ink-dim)] hover:border-[var(--color-border-strong)] hover:text-[var(--color-ink)] transition-colors"
            >
              <span
                aria-hidden
                className="pulse-dot inline-block h-1.5 w-1.5 rounded-full bg-[var(--color-accent)]"
              />
              Now in early access — view on GitHub
              <span aria-hidden>→</span>
            </a>
          </div>

          <h1
            id="hero-heading"
            className="float-in float-in-2 mt-8 max-w-[18ch] text-balance font-semibold leading-[1.02] tracking-[-0.035em]"
            style={{ fontSize: 'var(--text-hero)' }}
          >
            Real-time guardrails for AI agents.
          </h1>

          <p className="float-in float-in-3 mt-6 max-w-lg text-base leading-relaxed text-[var(--color-ink-dim)] sm:text-lg">
            Catch unsafe output before your customers see it. One call returns
            a safety verdict in milliseconds — allow, rewrite, block, or
            escalate — with the evidence to back it up.
          </p>

          <div className="float-in float-in-4 mt-10 flex flex-wrap items-center gap-3">
            <a href="#quickstart" className="btn-primary">
              Get started
              <ArrowRight />
            </a>
            <a href="/docs" className="btn-ghost">
              Read the docs
            </a>
          </div>
        </div>

        <HeroVisual />
      </div>
    </section>
  );
}

function ArrowRight() {
  return (
    <svg width="14" height="14" viewBox="0 0 14 14" fill="none" aria-hidden>
      <path
        d="M2 7H12M12 7L7.5 2.5M12 7L7.5 11.5"
        stroke="currentColor"
        strokeWidth="1.75"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}
