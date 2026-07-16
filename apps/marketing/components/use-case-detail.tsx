import { MarketingEventLink } from './marketing-event-link';
import type { UseCaseData } from '@/app/use-cases/content';

export function UseCaseDetail({ useCase }: { useCase: UseCaseData }) {
  return (
    <section
      id={useCase.slug}
      className="section use-case-detail"
      aria-labelledby={`${useCase.slug}-title`}
    >
      <header className="use-case-detail-heading">
        <div className="use-case-kicker">
          <span>{useCase.number}</span>
          <p>{useCase.eyebrow}</p>
        </div>
        <div>
          <h1 id={`${useCase.slug}-title`}>{useCase.title}</h1>
          <p>{useCase.summary}</p>
        </div>
      </header>

      <div className="use-case-context-grid">
        <ContextItem label="Trigger" value={useCase.trigger} />
        <ContextItem label="What breaks" value={useCase.failure} />
        <ContextItem label="Control point" value={useCase.control} accent />
      </div>

      <div className="use-case-flow" aria-label={`${useCase.eyebrow} control flow`}>
        {useCase.flow.map((node, index) => (
          <div key={node} className="use-case-flow-part">
            <div
              className={
                index === 1 || index === 2
                  ? 'use-case-flow-node use-case-flow-node-accent'
                  : 'use-case-flow-node'
              }
            >
              <span>0{index + 1}</span>
              <strong>{node}</strong>
            </div>
            {index < useCase.flow.length - 1 ? <i aria-hidden="true">→</i> : null}
          </div>
        ))}
      </div>

      {useCase.demo ? (
        <section className="use-case-demo" aria-labelledby={`${useCase.slug}-demo-title`}>
          <div className="use-case-demo-heading">
            <div>
              <p className="use-case-mini-label">Operator walkthrough</p>
              <h2 id={`${useCase.slug}-demo-title`}>See the control reach a real decision.</h2>
            </div>
            <p>{useCase.demo.caption}</p>
            <span className="use-case-demo-swipe">
              Swipe to inspect all four steps <b aria-hidden="true">→</b>
            </span>
          </div>
          <figure>
            <div
              className="use-case-demo-visual"
              tabIndex={0}
              aria-label="Scrollable four-step operator walkthrough"
            >
              <img
                src={useCase.demo.imageSrc}
                alt={useCase.demo.imageAlt}
                width="1672"
                height="941"
                loading="lazy"
              />
            </div>
            <figcaption>
              <span>Execution boundary</span>
              <strong>{useCase.demo.note}</strong>
            </figcaption>
          </figure>
        </section>
      ) : null}

      <div className="use-case-workflow-grid">
        <div>
          <p className="use-case-mini-label">How it works</p>
          <ol className="use-case-steps">
            {useCase.steps.map((step, index) => (
              <li key={step.label}>
                <span>{String(index + 1).padStart(2, '0')}</span>
                <div>
                  <small>{step.label}</small>
                  <h2>{step.title}</h2>
                  <p>{step.body}</p>
                </div>
              </li>
            ))}
          </ol>
        </div>

        <aside className="use-case-result-card" aria-label={`${useCase.eyebrow} result`}>
          <div>
            <p className="use-case-mini-label">What gets checked</p>
            <ul>
              {useCase.checks.map((check) => (
                <li key={check}>
                  <span aria-hidden="true">✓</span>
                  {check}
                </li>
              ))}
            </ul>
          </div>
          <div className="use-case-result">
            <p className="use-case-mini-label">Result</p>
            <h2>{useCase.result}</h2>
            <p>{useCase.resultDetail}</p>
          </div>
          <div>
            <p className="use-case-mini-label">Evidence kept</p>
            <div className="use-case-proof-list">
              {useCase.proof.map((item) => (
                <span key={item}>{item}</span>
              ))}
            </div>
          </div>
          <MarketingEventLink
            href={useCase.ctaHref}
            target="_blank"
            className="use-case-doc-link"
            event="docs_click"
            eventParams={{ page: useCase.href, location: 'detail', label: useCase.ctaLabel }}
          >
            {useCase.ctaLabel} <span aria-hidden="true">↗</span>
          </MarketingEventLink>
        </aside>
      </div>
    </section>
  );
}

function ContextItem({
  label,
  value,
  accent = false,
}: {
  label: string;
  value: string;
  accent?: boolean;
}) {
  return (
    <article className={accent ? 'use-case-context use-case-context-accent' : 'use-case-context'}>
      <p>{label}</p>
      <span>{value}</span>
    </article>
  );
}
