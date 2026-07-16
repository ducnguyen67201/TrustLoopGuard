import { GITHUB_URL } from '@/lib/github';
import { MarketingEventLink } from './marketing-event-link';

const FINANCIAL_CONTRACT_URL = `${GITHUB_URL}/blob/main/docs/concept/financial-authorization.md`;

const PROOF_POINTS = [
  {
    label: 'Apache-2.0',
    detail: 'Inspect every decision path',
    href: `${GITHUB_URL}/blob/main/LICENSE`,
  },
  {
    label: 'Self-hostable',
    detail: 'Rust runtime in your infrastructure',
    href: `${GITHUB_URL}#quickstart`,
  },
  {
    label: 'TypeScript · Python · Rust',
    detail: 'One generated decision contract',
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
      <div className="hero-signal" aria-label="TrustLoopGuard product status">
        <p>
          <span className="hero-signal-dot" aria-hidden="true" />
          Open-source control boundary
        </p>
        <span>Proposed action in. Typed decision out. Side effect stays on your side.</span>
      </div>

      <div className="hero-inner">
        <div className="hero-copy">
          <p className="eyebrow">Runtime control for production AI agents</p>
          <h1 id="hero-heading" className="hero-title">
            Stop AI agents
            <span>before they send, spend, or execute.</span>
          </h1>
          <p className="hero-sub">
            TrustLoopGuard checks a proposed output or action before it becomes a real side effect.
            Your runtime gets <strong>permit, deny, transform, require approval, or defer</strong>—
            plus a receipt showing why.
          </p>
          <div className="hero-actions">
            <MarketingEventLink
              href="/demo"
              className="button-primary h-12 px-6"
              event="demo_click"
              eventParams={{ page: '/', location: 'hero', label: 'Try the live refund demo' }}
            >
              <PlayIcon />
              Try the live refund demo
            </MarketingEventLink>
            <MarketingEventLink
              href="#how"
              className="button-secondary h-12 px-6"
              event="landing_cta_click"
              eventParams={{ page: '/', location: 'hero', label: 'See the control flow' }}
            >
              See the control flow
              <ArrowIcon />
            </MarketingEventLink>
          </div>
          <div className="hero-source-row">
            <span>No card. No signup. Runs against the real authorization path.</span>
            <MarketingEventLink
              href={GITHUB_URL}
              target="_blank"
              className="hero-source-link"
              event="github_click"
              eventParams={{ page: '/', location: 'hero', label: 'Inspect the source' }}
            >
              Inspect the source ↗
            </MarketingEventLink>
          </div>
        </div>

        <ControlBoundaryPreview />
      </div>

      <div className="proof-strip" aria-label="Inspectable product facts">
        {PROOF_POINTS.map((item, index) => (
          <a
            key={item.label}
            href={item.href}
            target="_blank"
            rel="noreferrer"
            className="proof-item"
          >
            <span className="proof-number">0{index + 1}</span>
            <span>
              <strong>{item.label}</strong>
              <small>{item.detail}</small>
            </span>
            <span className="proof-arrow" aria-hidden="true">
              ↗
            </span>
          </a>
        ))}
      </div>
    </section>
  );
}

function ControlBoundaryPreview() {
  return (
    <article
      className="control-preview"
      aria-label="Example control boundary: refund-bot proposes a 75 dollar refund, TrustLoopGuard requires approval, and execution does not start."
    >
      <header className="control-preview-header">
        <div>
          <span className="control-live-dot" aria-hidden="true" />
          Live control boundary
        </div>
        <code>POST /v1/financial/actions</code>
      </header>

      <div className="control-proposal">
        <span className="control-node-number">01</span>
        <div>
          <p>Agent proposes</p>
          <strong>issue_refund</strong>
          <dl>
            <div>
              <dt>principal</dt>
              <dd>refund-bot</dd>
            </div>
            <div>
              <dt>amount</dt>
              <dd>$75.00 USD</dd>
            </div>
          </dl>
        </div>
        <span className="control-proposal-state">Proposed</span>
      </div>

      <div className="control-gate">
        <div className="control-gate-rail" aria-hidden="true">
          <span />
        </div>
        <div className="control-gate-copy">
          <p>TrustLoopGuard checks</p>
          <ul>
            <li>
              <span>Authority</span>
              <strong>pass</strong>
            </li>
            <li>
              <span>Order evidence</span>
              <strong>pass</strong>
            </li>
            <li>
              <span>Refund policy</span>
              <strong className="control-check-held">approval</strong>
            </li>
          </ul>
        </div>
      </div>

      <div className="control-decision">
        <div className="control-decision-stamp">
          <small>Effect</small>
          <strong>REQUIRE</strong>
          <strong>APPROVAL</strong>
        </div>
        <div className="control-decision-copy">
          <p>Typed decision returned</p>
          <code>effect: require_approval</code>
          <div>
            <span>Execution not started</span>
            <span>Receipt reserved</span>
          </div>
        </div>
      </div>

      <footer className="control-preview-footer">
        <span>
          <StopIcon />
          Side effect stopped at the boundary
        </span>
        <strong>No Stripe call made</strong>
      </footer>
    </article>
  );
}

function PlayIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none" aria-hidden="true">
      <path d="M5.25 3.5 12 8l-6.75 4.5v-9Z" fill="currentColor" />
    </svg>
  );
}

function ArrowIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none" aria-hidden="true">
      <path
        d="M3 8h9M8.5 4.5 12 8l-3.5 3.5"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

function StopIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 14 14" fill="none" aria-hidden="true">
      <path
        d="M4.1 1.5h5.8l2.6 2.6v5.8l-2.6 2.6H4.1L1.5 9.9V4.1l2.6-2.6Z"
        stroke="currentColor"
        strokeWidth="1.2"
      />
      <path d="M4.25 7h5.5" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round" />
    </svg>
  );
}
