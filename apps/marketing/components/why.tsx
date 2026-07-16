import Link from 'next/link';
import {
  AGENT_SPENDING_CAPS_USE_CASE,
  EMAIL_USE_CASE,
  SHELL_COMMAND_USE_CASE,
} from '@/app/use-cases/content';

const USE_CASES = [
  {
    ...SHELL_COMMAND_USE_CASE,
    risk: 'A coding agent proposes a destructive command against a sensitive target.',
    control: 'Deny it or require exact-action approval before the executor runs it.',
  },
  {
    ...EMAIL_USE_CASE,
    risk: 'A customer-facing draft guarantees an outcome the business cannot promise.',
    control: 'Permit the safe draft or return policy-approved wording before delivery.',
  },
  {
    ...AGENT_SPENDING_CAPS_USE_CASE,
    risk: 'Routine spend, reviewable exceptions, and hard-cap breaches share one execution path.',
    control: 'Permit, hold, or deny the payment before a provider call can start.',
  },
] as const;

export function Why() {
  return (
    <section
      id="use-cases"
      aria-labelledby="use-cases-heading"
      className="section use-cases-section"
    >
      <div className="section-heading split-heading">
        <div>
          <p className="eyebrow">Where it fits</p>
          <h2 id="use-cases-heading" className="section-title">
            Start with an action you can recognize.
          </h2>
        </div>
        <p className="section-copy">
          These operator-ready paths use the same pattern: capture the proposal, evaluate policy
          outside the prompt, and return a decision before the action becomes real.
        </p>
      </div>

      <div className="use-case-grid">
        {USE_CASES.map((useCase) => (
          <article key={useCase.number}>
            <header>
              <span>{useCase.number}</span>
              <h3>{useCase.eyebrow}</h3>
            </header>
            <dl>
              <div>
                <dt>Failure</dt>
                <dd>{useCase.risk}</dd>
              </div>
              <div>
                <dt>Control</dt>
                <dd>{useCase.control}</dd>
              </div>
            </dl>
            <Link href={useCase.href} className="use-case-grid-link">
              Explore the walkthrough <span aria-hidden="true">→</span>
            </Link>
          </article>
        ))}
      </div>
      <Link href="/use-cases" className="use-case-grid-all">
        View all six use cases <span aria-hidden="true">→</span>
      </Link>
    </section>
  );
}
