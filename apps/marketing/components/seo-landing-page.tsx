import { BOOK_MEETING_URL, DOCS_URL } from '@/lib/github';
import {
  faqJsonLd,
  landingPageJsonLd,
  serializeJsonLd,
  type LandingPageData,
} from '@/lib/seo';
import { Ascii } from './ascii-art';
import { Footer } from './footer';
import { Eyebrow } from './how';
import { MarketingEventLink } from './marketing-event-link';
import { Nav } from './nav';
import { ScrollTopButton } from './scroll-top-button';

interface SeoLandingPageProps {
  page: LandingPageData;
}

export function SeoLandingPage({ page }: SeoLandingPageProps) {
  const pagePath = `/${page.slug}`;

  return (
    <>
      <Nav />
      <main>
        <section className="section border-b border-[var(--color-line)]">
          <Ascii
            name="shield"
            className="ascii-faint ascii-drift absolute right-8 top-10 hidden lg:block"
          />
          <div className="section-grid">
            <div>
              <Eyebrow>{page.eyebrow}</Eyebrow>
              <h1 className="section-title">{page.h1}</h1>
            </div>
            <div>
              <p className="section-copy">{page.intro}</p>
              <div className="mt-8 flex flex-col gap-3 sm:flex-row sm:flex-wrap">
                <MarketingEventLink
                  href={BOOK_MEETING_URL}
                  target="_blank"
                  rel="noreferrer"
                  className="button-accent h-11 px-5"
                  event="book_meeting_click"
                  eventParams={{ page: pagePath, location: 'landing_hero', label: page.primaryCta }}
                >
                  {page.primaryCta}
                </MarketingEventLink>
                <MarketingEventLink
                  href={DOCS_URL}
                  className="button-secondary h-11 px-5"
                  event="docs_click"
                  eventParams={{ page: pagePath, location: 'landing_hero', label: page.secondaryCta }}
                >
                  {page.secondaryCta}
                </MarketingEventLink>
              </div>
            </div>
          </div>
        </section>

        <section className="section">
          <div className="section-grid">
            <div>
              <Eyebrow>Failure modes</Eyebrow>
              <h2 className="section-title">{page.problemHeading}</h2>
            </div>
            <ul className="grid gap-px bg-[var(--color-line)]">
              {page.problems.map((problem) => (
                <li key={problem} className="cell flex gap-4 p-5 text-sm leading-7">
                  <span
                    className="mt-2 h-1.5 w-1.5 flex-none rounded-full bg-[var(--color-block)]"
                    aria-hidden="true"
                  />
                  <span>{problem}</span>
                </li>
              ))}
            </ul>
          </div>
        </section>

        <section className="section section-compact">
          <Eyebrow>Runtime check</Eyebrow>
          <h2 className="section-title max-w-3xl">{page.checkHeading}</h2>
          <div className="mt-10 grid gap-px bg-[var(--color-line)] md:grid-cols-3">
            {page.checks.map((check) => (
              <article key={check.title} className="cell p-6">
                <h3 className="text-xl font-semibold">{check.title}</h3>
                <p className="mt-4 text-sm leading-7 text-[var(--color-muted)]">{check.body}</p>
              </article>
            ))}
          </div>
        </section>

        <section className="section">
          <div className="card relative overflow-hidden p-8 md:p-10">
            <Ascii
              name="padlock"
              className="ascii-sm ascii-faint ascii-drift absolute -bottom-2 right-8 hidden md:block"
            />
            <div className="relative z-10 grid gap-8 lg:grid-cols-[0.8fr_1.2fr]">
              <div>
                <Eyebrow>Audit proof</Eyebrow>
                <h2 className="section-title">{page.proofHeading}</h2>
              </div>
              <div className="grid gap-3">
                {page.proof.map((item) => (
                  <div
                    key={item}
                    className="border-t border-[var(--color-line)] pt-3 font-mono text-sm text-[var(--color-muted)]"
                  >
                    <span className="text-[var(--color-accent)]">ok</span> {item}
                  </div>
                ))}
              </div>
            </div>
          </div>
        </section>

        <section className="section section-compact">
          <Eyebrow>Questions</Eyebrow>
          <h2 className="section-title max-w-3xl">What teams ask before they wire it in.</h2>
          <div className="mt-10 grid gap-4 md:grid-cols-2">
            {page.faq.map((item) => (
              <article key={item.question} className="card p-6">
                <h3 className="text-lg font-semibold">{item.question}</h3>
                <p className="mt-3 text-sm leading-7 text-[var(--color-muted)]">{item.answer}</p>
              </article>
            ))}
          </div>
        </section>

        <section className="section section-compact">
          <div className="cta-card p-8 md:p-10">
            <p className="eyebrow">Ship the check</p>
            <h2 className="mt-5 max-w-3xl text-3xl font-semibold leading-[1.08] tracking-tight md:text-5xl">
              Put a verdict between the agent and the action.
            </h2>
            <p className="mt-5 max-w-2xl text-base leading-7 text-[var(--color-muted)]">
              Bring a real checkout, refund, booking, or invoice flow. We will map the exact pre-action
              check and the audit record it should leave behind.
            </p>
            <div className="mt-8 flex flex-col gap-3 sm:flex-row sm:flex-wrap">
              <MarketingEventLink
                href={BOOK_MEETING_URL}
                target="_blank"
                rel="noreferrer"
                className="button-accent h-11 px-5"
                event="landing_cta_click"
                eventParams={{ page: pagePath, location: 'landing_bottom', label: 'Book a meeting' }}
              >
                Book a meeting
              </MarketingEventLink>
              <MarketingEventLink
                href={DOCS_URL}
                className="button-dark h-11 px-5"
                event="docs_click"
                eventParams={{ page: pagePath, location: 'landing_bottom', label: 'Read the docs' }}
              >
                Read the docs
              </MarketingEventLink>
            </div>
          </div>
        </section>
      </main>
      <Footer />
      <ScrollTopButton />
      <script
        type="application/ld+json"
        dangerouslySetInnerHTML={{ __html: serializeJsonLd(landingPageJsonLd(page)) }}
      />
      <script
        type="application/ld+json"
        dangerouslySetInnerHTML={{ __html: serializeJsonLd(faqJsonLd(page)) }}
      />
    </>
  );
}
