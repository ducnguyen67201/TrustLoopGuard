const STEPS = [
  {
    n: '01',
    title: 'Wrap your agent step',
    body: 'Call the SDK with the proposed output, the user prompt, and the policy to enforce. One function, three lines of code.',
  },
  {
    n: '02',
    title: 'Get scored in milliseconds',
    body: 'Prompt-injection, PII, jailbreak, and policy checks run in parallel. Single-digit milliseconds, even at p99.',
  },
  {
    n: '03',
    title: 'Act on the verdict',
    body: 'allow → ship · rewrite → use the safe version · block → refuse with a reason · escalate → route to a reviewer.',
  },
] as const;

export function How() {
  return (
    <section
      id="how"
      aria-labelledby="how-heading"
      className="relative mx-auto max-w-6xl px-6 py-32"
    >
      <Eyebrow>How it works</Eyebrow>
      <h2
        id="how-heading"
        className="mt-4 max-w-3xl text-balance font-medium leading-[1.02] tracking-[-0.03em]"
        style={{ fontSize: 'var(--text-display)' }}
      >
        A safety check on every step{' '}
        <span className="text-[var(--color-ink-dim)]">
          your agent takes.
        </span>
      </h2>
      <ol className="mt-16 grid gap-4 md:grid-cols-3">
        {STEPS.map((step) => (
          <li
            key={step.n}
            className="glass rounded-2xl p-8 transition-transform hover:-translate-y-0.5"
          >
            <span
              className="font-mono text-xs tracking-widest text-[var(--color-accent)]"
              aria-hidden
            >
              {step.n}
            </span>
            <h3 className="mt-4 text-xl font-medium tracking-tight">
              {step.title}
            </h3>
            <p className="mt-3 text-sm leading-relaxed text-[var(--color-ink-dim)]">
              {step.body}
            </p>
          </li>
        ))}
      </ol>
    </section>
  );
}

export function Eyebrow({ children }: { children: React.ReactNode }) {
  return (
    <span className="inline-flex items-center gap-2 text-[var(--text-eyebrow)] uppercase tracking-[0.22em] text-[var(--color-ink-mute)]">
      <span
        aria-hidden
        className="inline-block h-px w-6 bg-[var(--color-hairline-strong)]"
      />
      {children}
    </span>
  );
}
