import { BOOK_MEETING_URL, DOCS_URL, GITHUB_URL } from '@/lib/github';
import { Ascii } from './ascii-art';
import { MarketingEventLink } from './marketing-event-link';

export function Cta() {
  return (
    <section aria-labelledby="cta-heading" className="section">
      <div className="cta-card p-8 md:p-12">
        <Ascii
          name="globe"
          className="ascii-accent ascii-faint ascii-drift absolute -right-6 -top-4 hidden md:block"
        />
        <Ascii
          name="sparkC"
          className="ascii-pulse absolute bottom-6 right-[30%] hidden lg:block"
        />
        <div className="relative z-10">
          <p className="eyebrow">Ship the check</p>
          <h2
            id="cta-heading"
            className="mt-5 max-w-3xl text-3xl font-semibold leading-[1.08] tracking-tight md:text-5xl"
          >
            Add a spend cap and a signed audit trail before your agent pays.
          </h2>
          <p className="mt-5 max-w-2xl text-base leading-7 text-[var(--color-muted)]">
            Install the SDK, run the Rust service in your own VPC, and see every money-moving
            decision: what fired, why, and who approved it. Off-chain, on the rails you already use.
          </p>
          <div className="mt-8 flex flex-col gap-3 sm:flex-row sm:flex-wrap">
            <MarketingEventLink
              href={BOOK_MEETING_URL}
              target="_blank"
              rel="noreferrer"
              className="button-accent h-11 px-5"
              event="book_meeting_click"
              eventParams={{ page: '/', location: 'cta', label: 'Book a meeting' }}
            >
              Book a meeting
            </MarketingEventLink>
            <MarketingEventLink
              href={GITHUB_URL}
              className="button-invert h-11 px-5"
              event="github_click"
              eventParams={{ page: '/', location: 'cta', label: 'Clone on GitHub' }}
            >
              Clone on GitHub
            </MarketingEventLink>
            <MarketingEventLink
              href={DOCS_URL}
              className="button-dark h-11 px-5"
              event="docs_click"
              eventParams={{ page: '/', location: 'cta', label: 'Read the docs' }}
            >
              Read the docs
            </MarketingEventLink>
          </div>
        </div>
      </div>
    </section>
  );
}
