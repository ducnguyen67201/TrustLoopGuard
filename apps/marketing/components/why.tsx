import { Eyebrow } from './how';

const USE_CASES = [
  {
    title: 'Customer-support agents',
    body: 'Prevent leaked account data, unsafe advice, and policy-breaking replies.',
  },
  {
    title: 'Internal copilots',
    body: 'Stop prompt injection, secret exposure, and unauthorized tool use.',
  },
  {
    title: 'Workflow agents',
    body: 'Escalate high-risk actions before they trigger irreversible changes.',
  },
  {
    title: 'Developer platforms',
    body: 'Give users guardrails without forcing every team to build its own runtime layer.',
  },
] as const;

export function Why() {
  return (
    <section id="use-cases" aria-labelledby="use-cases-heading" className="section">
      <Eyebrow>06. Use cases</Eyebrow>
      <h2 id="use-cases-heading" className="section-title max-w-3xl">
        Built for teams putting agents in production.
      </h2>
      <div className="mt-12 grid gap-px bg-[var(--color-line)] md:grid-cols-2">
        {USE_CASES.map((useCase) => (
          <article key={useCase.title} className="bg-white p-6">
            <h3 className="text-xl font-semibold">{useCase.title}</h3>
            <p className="mt-4 text-sm leading-7 text-[var(--color-muted)]">{useCase.body}</p>
          </article>
        ))}
      </div>
    </section>
  );
}
