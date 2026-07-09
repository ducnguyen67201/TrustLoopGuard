import type { ReactNode } from 'react';

const AUTHORIZATION_CHECKS = [
  'Mandate & authority',
  'Trusted evidence',
  'Financial policy',
  'Spend window',
] as const;

const OUTCOMES = [
  { status: 'Authorized', detail: 'Continue to execution', className: 'journey-outcome-authorized' },
  { status: 'Held', detail: 'Wait for approval', className: 'journey-outcome-held' },
  { status: 'Denied', detail: 'Stop before execution', className: 'journey-outcome-denied' },
] as const;

export function ControlLoop() {
  return (
    <section id="how" aria-labelledby="how-heading" className="journey-section">
      <div className="section journey-intro">
        <p className="eyebrow">The authorization journey</p>
        <h2 id="how-heading" className="section-title">
          One proposed action. One controlled path to the real world.
        </h2>
        <p className="section-copy">
          Follow the blue route. The agent can propose an action, but it cannot cross the control
          boundary until TrustLoopGuard returns an actionable decision.
        </p>
      </div>

      <div className="journey-canvas">
        <img
          className="journey-art"
          src="/images/trustloop-authorization-journey.png"
          alt=""
          width="921"
          height="1707"
          loading="lazy"
        />

        <StoryCard className="journey-card-proposal" number="01" label="Agent proposes">
          <h3>The action has not happened yet.</h3>
          <p>
            Your runtime submits the intended action and its context before any side effect reaches
            a user, tool, or payment rail.
          </p>
          <dl className="journey-action">
            <div><dt>action</dt><dd>issue_refund</dd></div>
            <div><dt>amount</dt><dd>$75.00 USD</dd></div>
            <div><dt>status</dt><dd>proposed</dd></div>
          </dl>
        </StoryCard>

        <StoryCard className="journey-card-checks" number="02" label="Control boundary">
          <h3>TrustLoopGuard holds the action at the gate.</h3>
          <p>Checks run against durable runtime context—not just another instruction in the prompt.</p>
          <ul className="journey-checks">
            {AUTHORIZATION_CHECKS.map((check) => (
              <li key={check}><span aria-hidden="true">✓</span>{check}</li>
            ))}
          </ul>
        </StoryCard>

        <StoryCard className="journey-card-decision" number="03" label="Decision point">
          <h3>Every route is explicit.</h3>
          <p>The response tells the caller exactly what may happen next—and why.</p>
          <div className="journey-outcomes">
            {OUTCOMES.map((outcome) => (
              <div key={outcome.status} className={outcome.className}>
                <strong>{outcome.status}</strong>
                <span>{outcome.detail}</span>
              </div>
            ))}
          </div>
        </StoryCard>

        <StoryCard className="journey-card-execute" number="04" label="Authorized only">
          <h3>Your runtime performs the action.</h3>
          <p>
            Held actions wait. Denied actions stop. Only an authorized action continues through the
            gate to your existing execution code.
          </p>
          <code>executeAction(action.id)</code>
        </StoryCard>

        <StoryCard className="journey-card-proof" number="05" label="Evidence after action">
          <h3>The loop closes with proof.</h3>
          <p>
            A decision receipt records the authorization before execution. An execution receipt
            records what actually happened after it.
          </p>
          <div className="journey-receipts">
            <span>Decision receipt</span><i aria-hidden="true">→</i><span>Execution receipt</span>
          </div>
        </StoryCard>
      </div>

      <p className="journey-footnote">
        <strong>Non-financial actions:</strong> the same boundary returns <code>allow</code>,{' '}
        <code>block</code>, <code>rewrite</code>, or <code>escalate</code> before execution.
      </p>
    </section>
  );
}

function StoryCard({
  className,
  number,
  label,
  children,
}: {
  className: string;
  number: string;
  label: string;
  children: ReactNode;
}) {
  return (
    <article className={`journey-card ${className}`}>
      <div className="journey-card-label"><span>{number}</span><small>{label}</small></div>
      {children}
    </article>
  );
}
