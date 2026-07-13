import type { Metadata } from 'next';
import Link from 'next/link';
import { Footer } from '@/components/footer';
import { MarketingEventLink } from '@/components/marketing-event-link';
import { Nav } from '@/components/nav';
import { ScrollTopButton } from '@/components/scroll-top-button';
import { BOOK_MEETING_URL } from '@/lib/github';
import { absoluteUrl } from '@/lib/seo';
import { USE_CASES } from './content';

const PAGE_PATH = '/use-cases';
const PAGE_TITLE = 'Use Cases for AI Spend and Action Authorization | TrustLoopGuard';
const PAGE_DESCRIPTION =
  'See how TrustLoopGuard controls AI inference spend, authorizes x402 payments, guards irreversible actions, and checks email sends before execution.';

export const metadata: Metadata = {
  title: { absolute: PAGE_TITLE },
  description: PAGE_DESCRIPTION,
  alternates: { canonical: PAGE_PATH },
  openGraph: {
    title: PAGE_TITLE,
    description: PAGE_DESCRIPTION,
    url: absoluteUrl(PAGE_PATH),
    type: 'website',
  },
  twitter: {
    card: 'summary_large_image',
    title: PAGE_TITLE,
    description: PAGE_DESCRIPTION,
  },
};

export default function Page() {
  return (
    <>
      <Nav />
      <main>
        <UseCasesHero />
        <UseCaseIndex />
        <BoundarySection />
        <UseCasesCta />
      </main>
      <Footer />
      <ScrollTopButton />
    </>
  );
}

function UseCasesHero() {
  return (
    <section className="use-cases-hero" aria-labelledby="use-cases-title">
      <div className="use-cases-hero-inner">
        <div>
          <p className="eyebrow">Use cases</p>
          <h1 id="use-cases-title">Put the stop button before the spend.</h1>
        </div>
        <div className="use-cases-hero-copy">
          <p>
            TrustLoopGuard is the control layer between an AI system&apos;s proposed action and the
            provider, payment rail, or tool that makes it real.
          </p>
          <p>
            Start with one of four boundaries we support today: model usage, x402 payments, a
            consequential workflow action, or an external email send.
          </p>
          <MarketingEventLink
            href={BOOK_MEETING_URL}
            target="_blank"
            className="button-primary h-12 px-6"
            event="book_meeting_click"
            eventParams={{
              page: PAGE_PATH,
              location: 'use_cases_hero',
              label: 'Map your control point',
            }}
          >
            Map your control point <span aria-hidden="true">↗</span>
          </MarketingEventLink>
        </div>
      </div>
    </section>
  );
}

function UseCaseIndex() {
  return (
    <nav className="use-case-index" aria-label="TrustLoopGuard use cases">
      {USE_CASES.map((useCase) => (
        <Link key={useCase.slug} href={useCase.href} className="use-case-index-card">
          <span>{useCase.number}</span>
          <strong>{useCase.eyebrow}</strong>
          <h2>{useCase.title}</h2>
          <p>{useCase.summary}</p>
          <small>{useCase.result}</small>
          <i>
            Explore <b aria-hidden="true">→</b>
          </i>
        </Link>
      ))}
    </nav>
  );
}

function BoundarySection() {
  const boundaries = [
    [
      'Not the model provider',
      'We meter and control the call. Your chosen provider still runs the model.',
    ],
    [
      'Not the payment rail',
      'We authorize the spend. x402, Stripe, PayPal, cards, or your existing system still move the money.',
    ],
    [
      'Not another workflow system',
      'We guard the commit point. Your agent, AP tool, support stack, or application still owns the workflow.',
    ],
  ] as const;

  return (
    <section className="use-case-boundary-section" aria-labelledby="boundary-heading">
      <div className="section">
        <div className="split-heading section-heading">
          <div>
            <p className="eyebrow eyebrow-light">One role, clearly bounded</p>
            <h2 id="boundary-heading" className="section-title">
              The rail executes. TrustLoopGuard decides.
            </h2>
          </div>
          <p className="section-copy">
            That separation makes the control portable. You can change providers, rails, or workflow
            tools without moving the policy and evidence boundary back into the agent prompt.
          </p>
        </div>
        <div className="use-case-boundary-grid">
          {boundaries.map(([title, body], index) => (
            <article key={title}>
              <span>0{index + 1}</span>
              <h3>{title}</h3>
              <p>{body}</p>
            </article>
          ))}
        </div>
      </div>
    </section>
  );
}

function UseCasesCta() {
  return (
    <section className="section use-case-page-cta" aria-labelledby="use-case-cta-heading">
      <div>
        <p className="eyebrow">Start with one boundary</p>
        <h2 id="use-case-cta-heading">Which action are you still afraid to automate?</h2>
      </div>
      <div>
        <p>
          Bring the provider call, x402 payment, refund, invoice, payout, email send, or tool
          action. We will map what must be checked, where the hold belongs, and what proof you need
          afterward.
        </p>
        <MarketingEventLink
          href={BOOK_MEETING_URL}
          target="_blank"
          className="button-invert h-12 px-6"
          event="book_meeting_click"
          eventParams={{
            page: PAGE_PATH,
            location: 'use_cases_cta',
            label: 'Talk through the action',
          }}
        >
          Talk through the action <span aria-hidden="true">↗</span>
        </MarketingEventLink>
      </div>
    </section>
  );
}
