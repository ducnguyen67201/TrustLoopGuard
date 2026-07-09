import { GITHUB_URL } from '@/lib/github';
import { MarketingEventLink } from './marketing-event-link';

const EXAMPLE_ACTION_ID = '0195f2a4-7c31-7a4e-a50e-2d36fb38ec42';
const FINANCIAL_CONTRACT_URL = `${GITHUB_URL}/blob/main/docs/concept/financial-authorization.md`;

const PROOF_POINTS = [
  {
    label: 'Apache-2.0',
    detail: 'Open source',
    href: `${GITHUB_URL}/blob/main/LICENSE`,
  },
  {
    label: 'Your infrastructure',
    detail: 'Self-hostable Rust runtime',
    href: `${GITHUB_URL}#quickstart`,
  },
  {
    label: 'TypeScript · Python · Rust',
    detail: 'One generated wire contract',
    href: `${GITHUB_URL}#sdk-quickstarts`,
  },
  {
    label: 'Decision + receipt',
    detail: 'Authorization before, proof after',
    href: FINANCIAL_CONTRACT_URL,
  },
] as const;

export function Hero() {
  return (
    <section id="product" className="hero" aria-labelledby="hero-heading">
      <div className="founder-proof-bar" aria-label="Founder production experience">
        <p className="founder-proof-label"><span aria-hidden="true" />Founder proof</p>
        <p className="founder-proof-statement">
          <strong>Tested AI voice agents in production.</strong>
          <span>Saw how they failed every day.</span>
        </p>
        <a href="#trust">Why this exists <span aria-hidden="true">↓</span></a>
      </div>
      <div className="hero-inner">
        <div className="hero-copy">
          <p className="eyebrow">Runtime control for production AI agents</p>
          <h1 id="hero-heading" className="hero-title">
            Assume the agent will fail.
            <span>Control what happens next.</span>
          </h1>
          <p className="hero-sub">
            TrustLoopGuard sits between an agent&apos;s intent and execution. It returns an actionable
            decision before the side effect—and gives money-bearing actions durable authorization
            state and receipts.
          </p>
          <div className="hero-actions">
            <MarketingEventLink
              href="#how"
              className="button-primary h-12 px-6"
              event="landing_cta_click"
              eventParams={{ page: '/', location: 'hero', label: 'See how it works' }}
            >
              See how it works
              <ArrowIcon />
            </MarketingEventLink>
            <MarketingEventLink
              href={GITHUB_URL}
              target="_blank"
              className="button-secondary h-12 px-6"
              event="github_click"
              eventParams={{ page: '/', location: 'hero', label: 'Inspect the source' }}
            >
              Inspect the source
            </MarketingEventLink>
          </div>
        </div>

        <AuthorizationRecord />
      </div>

      <div className="proof-strip" aria-label="Inspectable product facts">
        {PROOF_POINTS.map((item, index) => (
          <a key={item.label} href={item.href} target="_blank" rel="noreferrer" className="proof-item">
            <span className="proof-number">0{index + 1}</span>
            <span>
              <strong>{item.label}</strong>
              <small>{item.detail}</small>
            </span>
            <span className="proof-arrow" aria-hidden="true">↗</span>
          </a>
        ))}
      </div>
    </section>
  );
}

function AuthorizationRecord() {
  return (
    <article className="decision-record" aria-label="Example TrustLoopGuard financial authorization">
      <header className="record-header">
        <div>
          <p>Example authorization</p>
          <strong>POST /v1/financial/actions</strong>
        </div>
        <span className="record-state record-state-held">Held</span>
      </header>

      <ol className="record-chain">
        <li>
          <span className="record-step">01</span>
          <div>
            <p className="record-label">Proposed action</p>
            <strong>refund · issue_refund</strong>
            <dl className="record-fields">
              <div><dt>principal</dt><dd>refund-bot</dd></div>
              <div><dt>amount</dt><dd>$75.00 USD</dd></div>
            </dl>
          </div>
        </li>
        <li>
          <span className="record-step">02</span>
          <div>
            <p className="record-label">Authorization checks</p>
            <strong>Mandate scope passed</strong>
            <p className="record-note">Financial policy requires approval above $50.</p>
          </div>
        </li>
        <li>
          <span className="record-step">03</span>
          <div>
            <p className="record-label">Authorization</p>
            <div className="record-verdict-row">
              <strong className="verdict-held">HELD</strong>
              <span>finance approval required</span>
            </div>
          </div>
        </li>
        <li>
          <span className="record-step">04</span>
          <div>
            <p className="record-label">Execution</p>
            <strong>NOT STARTED</strong>
            <code>action_id · {EXAMPLE_ACTION_ID}</code>
          </div>
        </li>
      </ol>

      <footer className="record-footer">
        <span>Example data · authentic wire fields</span>
        <a href={FINANCIAL_CONTRACT_URL} target="_blank" rel="noreferrer">
          Inspect authorization ↗
        </a>
      </footer>
    </article>
  );
}

function ArrowIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none" aria-hidden="true">
      <path d="M3 8h9M8.5 4.5 12 8l-3.5 3.5" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}
