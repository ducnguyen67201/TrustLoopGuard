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
      className="border-b border-[var(--color-border)]"
    >
      <div className="mx-auto max-w-6xl px-6 py-24 sm:py-32">
        <span className="eyebrow">How it works</span>
        <h2
          id="how-heading"
          className="mt-5 max-w-3xl text-balance font-semibold leading-[1.05] tracking-[-0.025em]"
          style={{ fontSize: 'var(--text-display)' }}
        >
          A safety check on every step your agent takes.
        </h2>

        <ol className="mt-14 grid gap-px overflow-hidden rounded-xl border border-[var(--color-border)] bg-[var(--color-border)] md:grid-cols-3">
          {STEPS.map((step) => (
            <li
              key={step.n}
              className="bg-[var(--color-surface)] p-8 transition-colors hover:bg-[var(--color-canvas-soft)]"
            >
              <span
                className="font-mono text-xs tracking-widest text-[var(--color-ink-mute)]"
                aria-hidden
              >
                {step.n}
              </span>
              <h3 className="mt-4 text-lg font-medium tracking-tight">
                {step.title}
              </h3>
              <p className="mt-3 text-sm leading-relaxed text-[var(--color-ink-dim)]">
                {step.body}
              </p>
            </li>
          ))}
        </ol>
      </div>
    </section>
  );
}
