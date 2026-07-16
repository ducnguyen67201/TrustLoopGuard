import Link from 'next/link';
import {
  AGENT_SPENDING_CAPS_USE_CASE,
  EMAIL_USE_CASE,
  SHELL_COMMAND_USE_CASE,
} from '@/app/use-cases/content';
import { UseCaseShowcase } from './use-case-showcase';

const FEATURED_USE_CASES = [
  SHELL_COMMAND_USE_CASE,
  EMAIL_USE_CASE,
  AGENT_SPENDING_CAPS_USE_CASE,
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
          Choose a real action and follow it through the same control loop: capture the proposal,
          evaluate policy outside the prompt, return an explicit decision, then let the existing
          runtime act.
        </p>
      </div>

      <UseCaseShowcase useCases={FEATURED_USE_CASES} />

      <Link href="/use-cases" className="use-case-showcase-all">
        View all six use cases <span aria-hidden="true">→</span>
      </Link>
    </section>
  );
}
