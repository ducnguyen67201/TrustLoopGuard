import { GITHUB_URL } from '@/lib/github';

const PRINCIPLES = [
  {
    number: '01',
    title: 'Policy before action',
    body: 'The check runs on the proposed output or action—not after the side effect has already happened.',
  },
  {
    number: '02',
    title: 'A reason, not a mystery',
    body: 'Every decision returns the verdict, the policies that fired, and a human-readable reason.',
  },
  {
    number: '03',
    title: 'Inspectable by default',
    body: 'The runtime is open source. The wire contract, policy engine, and SDK behavior are available to review.',
  },
] as const;

export function TrustStory() {
  return (
    <section id="trust" aria-labelledby="trust-heading" className="section trust-section">
      <div className="section-heading trust-heading">
        <p className="eyebrow">Why this exists</p>
        <h2 id="trust-heading" className="section-title">
          Built after watching agents fail in production.
        </h2>
      </div>

      <div className="trust-grid">
        <figure className="founder-note">
          <blockquote>
            “At an AI voice-testing company, I watched production find the failures demos missed.
            I built TrustLoopGuard around that lesson: assume the edge case is coming, put policy
            before action, and leave evidence behind.”
          </blockquote>
          <figcaption>
            <span className="founder-avatar" aria-hidden="true">D</span>
            <span><strong>Duc</strong><small>Founder, TrustLoopGuard</small></span>
          </figcaption>
        </figure>

        <div className="principles" aria-label="TrustLoopGuard design principles">
          <div className="principles-intro">
            <span>Trust is evidence</span>
            <a href={GITHUB_URL} target="_blank" rel="noreferrer">Review the repository ↗</a>
          </div>
          {PRINCIPLES.map((principle) => (
            <article key={principle.number} className="principle">
              <span>{principle.number}</span>
              <div>
                <h3>{principle.title}</h3>
                <p>{principle.body}</p>
              </div>
            </article>
          ))}
        </div>
      </div>
    </section>
  );
}
