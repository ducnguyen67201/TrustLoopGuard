import { VerdictCard } from './verdict-card';
import { HeroVisual } from './hero-visual';
import { GITHUB_URL } from '@/lib/github';

export function Hero() {
  return (
    <section
      aria-labelledby="hero-heading"
      className="relative isolate overflow-hidden pt-36 pb-20 sm:pt-44 sm:pb-28"
    >
      <div className="relative mx-auto grid max-w-6xl gap-12 px-6 lg:grid-cols-[1.05fr_1fr] lg:items-center lg:gap-16">
        <div>
          <div className="float-in float-in-1 inline-flex items-center gap-2 rounded-full glass px-3 py-1 text-[var(--text-eyebrow)] uppercase tracking-[0.2em] text-[var(--color-ink-dim)]">
            <span
              aria-hidden
              className="pulse-dot inline-block h-1.5 w-1.5 rounded-full bg-[var(--color-accent)]"
            />
            Real-time guardrail runtime
          </div>

          <h1
            id="hero-heading"
            className="float-in float-in-2 mt-8 max-w-[16ch] text-balance font-sans font-semibold leading-[0.95] tracking-[-0.045em]"
            style={{ fontSize: 'var(--text-hero)' }}
          >
            Think safer.
            <br />
            Stay aligned.
            <br />
            <span className="highlight-pill">Ship faster.</span>
          </h1>

          <p className="float-in float-in-3 mt-8 max-w-xl text-lg leading-relaxed text-[var(--color-ink-dim)]">
            Catch unsafe output from your AI agents before your customers
            ever see it. One call returns a safety verdict in milliseconds —
            allow, rewrite, block, or escalate — with the evidence to back
            it up.
          </p>

          <div className="float-in float-in-4 mt-10 flex flex-wrap items-center gap-3">
            <a
              href="#quickstart"
              className="inline-flex items-center gap-2 rounded-full bg-[var(--color-ink)] px-6 py-3 text-sm font-medium text-white hover:bg-[var(--color-accent)] transition-colors"
            >
              Start free trial
              <ArrowRight />
            </a>
            <a
              href={GITHUB_URL}
              className="inline-flex items-center gap-2 rounded-full glass-tight px-6 py-3 text-sm font-medium text-[var(--color-ink)] hover:bg-white/80 transition-colors"
            >
              <PlayIcon />
              See how it works
            </a>
          </div>
        </div>

        <HeroVisual />
      </div>

      <div className="relative mx-auto max-w-6xl px-6">
        {/* Verdict bento preview */}
        <div className="float-in float-in-5 mt-20 grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
          <VerdictCard
            verdict="allow"
            label="Allow"
            sample="Ship the response unchanged."
            latency="1.2ms"
          />
          <VerdictCard
            verdict="rewrite"
            label="Rewrite"
            sample="Redact the secret, keep the answer."
            latency="3.4ms"
          />
          <VerdictCard
            verdict="block"
            label="Block"
            sample="Refuse — prompt-injection detected."
            latency="0.9ms"
          />
          <VerdictCard
            verdict="escalate"
            label="Escalate"
            sample="Page a human reviewer."
            latency="2.1ms"
          />
        </div>
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

function PlayIcon() {
  return (
    <svg width="12" height="12" viewBox="0 0 12 12" fill="none" aria-hidden>
      <circle cx="6" cy="6" r="5" stroke="currentColor" strokeWidth="1.5" />
      <path d="M5 4.5L8 6L5 7.5V4.5Z" fill="currentColor" />
    </svg>
  );
}
