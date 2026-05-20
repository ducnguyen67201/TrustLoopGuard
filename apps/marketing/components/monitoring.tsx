import { Eyebrow } from './how';

const METRICS = ['Blocked actions', 'Escalations', 'Policy failures'] as const;

export function Monitoring() {
  return (
    <section
      id="monitoring"
      aria-labelledby="monitoring-heading"
      className="monitoring-band border-y border-[var(--color-line)]"
    >
      <div className="monitoring-inner">
        <div className="monitoring-copy">
          <Eyebrow>05. Monitoring</Eyebrow>
          <h2 id="monitoring-heading" className="section-title">
            Track failed policies before users are exposed.
          </h2>
          <p className="mt-5 max-w-xl text-lg leading-8 text-white/68">
            See which actions were blocked, rewritten, or escalated, with a trace for why.
          </p>
          <div className="mt-8 grid gap-3">
            {METRICS.map((title) => (
              <div key={title} className="monitoring-row">
                <span aria-hidden="true" />
                <strong>{title}</strong>
              </div>
            ))}
          </div>
        </div>
        <div className="monitoring-visual" aria-hidden="true">
          <img src="/monitoring-dashboard.png" alt="" aria-hidden="true" className="w-full" />
        </div>
      </div>
    </section>
  );
}
