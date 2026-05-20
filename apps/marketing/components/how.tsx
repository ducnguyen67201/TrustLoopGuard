const PROBLEMS = [
  'Unsafe output can reach users before a review happens.',
  'Teams cannot explain why a response or tool call was allowed.',
  'Policies drift across prototypes, SDKs, and production services.',
] as const;

const LOOP = [
  {
    n: '01',
    title: 'Agent proposes an action',
    body: 'Your app or proxy layer captures the prompt, proposed output, and policy context before delivery.',
  },
  {
    n: '02',
    title: 'TrustLoopGuard checks it',
    body: 'The Rust API evaluates policy and runtime checks against the proposal.',
  },
  {
    n: '03',
    title: 'Your app handles the verdict',
    body: 'Continue, use the rewrite, block with a reason, or escalate with context attached. You still own delivery.',
  },
  {
    n: '04',
    title: 'Every decision is traced',
    body: 'The dashboard can show what happened, which policy fired, and how the app responded.',
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
        <div className="section-grid">
          <div>
            <Eyebrow>01. The problem</Eyebrow>
            <h2 id="problem-heading" className="section-title">
              Agents are starting to take real actions.
            </h2>
          </div>
          <div>
            <p className="section-copy">
              They send emails, call tools, query private data, trigger workflows, and speak
              directly to customers. A prompt filter is not enough once the agent is deciding what
              to do next.
            </p>
            <ul className="mt-8 divide-y divide-[var(--color-line)] border-y border-[var(--color-line)]">
              {PROBLEMS.map((problem) => (
                <li key={problem} className="py-4 text-base leading-7">
                  {problem}
                </li>
              ))}
            </ul>
          </div>
        </div>
      </section>

      <section id="loop" aria-labelledby="loop-heading" className="section">
        <Eyebrow>02. Runtime loop</Eyebrow>
        <h2 id="loop-heading" className="section-title max-w-3xl">
          Add one check before your agent acts.
        </h2>
        <div className="mt-12 grid gap-px bg-[var(--color-line)] md:grid-cols-2 lg:grid-cols-4">
          {LOOP.map((step) => (
            <article key={step.n} className="bg-white p-6">
              <p className="font-mono text-sm text-[var(--color-accent)]">{step.n}</p>
              <h3 className="mt-5 text-xl font-semibold">{step.title}</h3>
              <p className="mt-4 text-sm leading-7 text-[var(--color-muted)]">{step.body}</p>
            </article>
          ))}
        </div>
      </section>
    </>
  );
}

export function Eyebrow({ children }: { children: React.ReactNode }) {
  return <p className="eyebrow">{children}</p>;
}
