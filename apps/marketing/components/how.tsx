import { Ascii } from './ascii-art';

const PROBLEMS = [
  'A $4,820 charge for the wrong item clears the card before anyone looks.',
  'A re-sent checkout link books the same trip twice.',
  'Finance asks who approved it, and no one can prove what happened.',
] as const;

const LOOP = [
  {
    n: '01',
    title: 'Agent proposes a money action',
    body: 'Your app captures the proposed charge, refund, or booking before it fires.',
  },
  {
    n: '02',
    title: 'TrustLoopGuard checks it',
    body: 'The Rust engine scores it against your spend caps, per-merchant limits, and policy in under 10ms.',
  },
  {
    n: '03',
    title: 'Your app handles the verdict',
    body: 'Allow, cap to the limit, block with a reason, or escalate for approval. Your agent still owns the action.',
  },
  {
    n: '04',
    title: 'Every decision is signed',
    body: 'A hash-chained trace records what was paid, which limit fired, and who approved it.',
  },
] as const;

export function How() {
  return (
    <>
      <section
        id="problem"
        aria-labelledby="problem-heading"
        className="section border-b border-[var(--color-line)]"
      >
        <Ascii name="sparkB" className="ascii-pulse absolute right-10 top-10 hidden md:block" />
        <div className="section-grid">
          <div>
            <Eyebrow>01 · The problem</Eyebrow>
            <h2 id="problem-heading" className="section-title">
              Your agent can move money now.
            </h2>
          </div>
          <div>
            <p className="section-copy">
              It charges cards, books trips, pays invoices, and issues refunds. A prompt filter
              can&rsquo;t stop a wrong or duplicate payment. You need a check on the action itself,
              with the amount attached, before it fires.
            </p>
            <ul className="mt-8 border-t border-[var(--color-line)]">
              {PROBLEMS.map((problem) => (
                <li
                  key={problem}
                  className="flex gap-4 border-b border-[var(--color-line)] py-4 text-base leading-7"
                >
                  <span
                    className="mt-2 h-1.5 w-1.5 flex-none rounded-full bg-[var(--color-block)]"
                    aria-hidden="true"
                  />
                  {problem}
                </li>
              ))}
            </ul>
          </div>
        </div>
      </section>

      <section id="loop" aria-labelledby="loop-heading" className="section">
        <Ascii
          name="padlock"
          className="ascii-faint ascii-drift absolute right-6 top-14 hidden lg:block"
        />
        <Eyebrow>02 · Runtime loop</Eyebrow>
        <h2 id="loop-heading" className="section-title max-w-3xl">
          Add one check before your agent moves money.
        </h2>
        <ol className="mt-12 grid gap-px bg-[var(--color-line)] md:grid-cols-2 lg:grid-cols-4">
          {LOOP.map((step) => (
            <li key={step.n} className="cell p-6">
              <p className="font-mono text-sm text-[var(--color-accent)]">{step.n}</p>
              <h3 className="mt-5 text-lg font-semibold leading-snug">{step.title}</h3>
              <p className="mt-4 text-sm leading-7 text-[var(--color-muted)]">{step.body}</p>
            </li>
          ))}
        </ol>
      </section>
    </>
  );
}

export function Eyebrow({ children }: { children: React.ReactNode }) {
  return <p className="eyebrow">{children}</p>;
}
