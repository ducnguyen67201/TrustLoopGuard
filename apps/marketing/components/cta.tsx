import { BOOK_MEETING_URL, DOCS_URL, GITHUB_URL } from '@/lib/github';
import { MarketingEventLink } from './marketing-event-link';

export function Cta() {
  return (
    <section aria-labelledby="cta-heading" className="section cta-section">
      <div className="cta-card">
        <div>
          <p className="eyebrow eyebrow-light">Start with a real failure path</p>
          <h2 id="cta-heading">Bring the agent action you are least comfortable shipping.</h2>
        </div>
        <div>
          <p>
            We will map the event, the policy boundary, and the decision your runtime needs before
            that action reaches a user or tool.
          </p>
          <div className="cta-actions">
            <MarketingEventLink
              href={BOOK_MEETING_URL}
              target="_blank"
              className="button-invert h-12 px-6"
              event="book_meeting_click"
              eventParams={{ page: '/', location: 'cta', label: 'Talk through a failure path' }}
            >
              Talk through a failure path
            </MarketingEventLink>
            <MarketingEventLink
              href={GITHUB_URL}
              target="_blank"
              className="button-dark h-12 px-6"
              event="github_click"
              eventParams={{ page: '/', location: 'cta', label: 'Review the source' }}
            >
              Review the source
            </MarketingEventLink>
            <MarketingEventLink
              href={DOCS_URL}
              target="_blank"
              className="cta-text-link"
              event="docs_click"
              eventParams={{ page: '/', location: 'cta', label: 'Read the docs' }}
            >
              Read the docs ↗
            </MarketingEventLink>
          </div>
        </div>
      </div>
    </section>
  );
}
