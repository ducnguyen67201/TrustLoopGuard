import { BOOK_MEETING_URL } from '@/lib/github';
import { Ascii } from './ascii-art';
import { HeroCard } from './hero-card';
import { MarketingEventLink } from './marketing-event-link';
import { TrustBand } from './trust-band';

interface LedgerRecord {
  time: string;
  verdict: 'allow' | 'rewrite' | 'block' | 'escalate';
  label: string;
  merchant: string;
  amount: string;
  hash: string;
}

const VERDICT_COLOR: Record<LedgerRecord['verdict'], string> = {
  allow: 'var(--color-allow)',
  rewrite: 'var(--color-rewrite)',
  block: 'var(--color-block)',
  escalate: 'var(--color-escalate)',
};

const LEDGER: LedgerRecord[] = [
  {
    time: '14:31:58',
    verdict: 'allow',
    label: 'ALLOW',
    merchant: 'Stripe payout · ops',
    amount: '$1,200.00',
    hash: '#a91f→c4',
  },
  {
    time: '14:31:12',
    verdict: 'rewrite',
    label: 'CAP',
    merchant: 'AWS invoice (capped from $6,400)',
    amount: '$5,000.00',
    hash: '#c4e0→9f',
  },
  {
    time: '14:30:41',
    verdict: 'escalate',
    label: 'HOLD',
    merchant: 'Refund · order #8842',
    amount: '$640.00',
    hash: '#9f3a→2b',
  },
];

export function Hero() {
  return (
    <section id="demo" className="hero">
      <div className="hero-media" aria-hidden="true">
        <video
          className="hero-video"
          autoPlay
          loop
          muted
          playsInline
          preload="metadata"
          poster="/proxy-flow-visual.png"
        >
          <source src="/trustloop-launch-demo.mp4" type="video/mp4" />
        </video>
        <div className="hero-scrim" />
        <Ascii name="sparkC" className="ascii-drift absolute right-[6%] top-20 hidden md:block" />
        <Ascii
          name="sparkB"
          className="ascii-sm ascii-pulse absolute left-[44%] top-10 hidden lg:block"
        />
        <Ascii
          name="sparkA"
          className="ascii-drift absolute bottom-[30%] left-[4%] hidden xl:block"
        />
      </div>
      <div className="hero-inner">
        {/* Above-the-fold answers: what is it / is it for me / what do I do next.
            Headline spans the full width; the card terminal sits right below it. */}
        <div>
          <p className="eyebrow">Spend caps for AI agents</p>
          <h1 className="hero-title">
            <span>Cap what your agent spends</span>
            <em>PROVE</em>
            <span>what it did</span>
          </h1>
        </div>

        <div className="hero-copy">
          <p className="hero-sub">
            Your agent proposes a charge. TrustLoopGuard answers{' '}
            <strong className="text-allow">allow</strong> /{' '}
            <strong className="text-rewrite">cap</strong> /{' '}
            <strong className="text-block">block</strong> /{' '}
            <strong className="text-escalate">escalate</strong> in under 10ms — and signs a record
            of why.
          </p>

          {/* ponytail: one loud action. GitHub already lives in the nav with stars. */}
          <div className="mt-8 flex flex-col items-stretch gap-3 sm:flex-row sm:flex-wrap sm:items-center sm:justify-center">
            <MarketingEventLink
              href="#quickstart"
              className="button-accent h-12 px-6"
              event="install_sdk_click"
              eventParams={{ page: '/', location: 'hero', label: 'Install the SDK' }}
            >
              Install the SDK
            </MarketingEventLink>
            <MarketingEventLink
              href={BOOK_MEETING_URL}
              target="_blank"
              rel="noreferrer"
              className="button-secondary h-12 px-6"
              event="book_meeting_click"
              eventParams={{ page: '/', location: 'hero', label: 'Book a meeting' }}
            >
              Book a meeting
            </MarketingEventLink>
          </div>
        </div>

        {/* the product in one image: the card your agent can't overrun */}
        <HeroCard />

        <a href="#problem" className="scroll-cue" aria-label="Scroll to the next section">
          ▼
        </a>
      </div>

      <TrustBand />

      {/* below the fold on purpose — above it only the pitch and the card */}
      <div className="ledger grid gap-6 py-10 lg:grid-cols-[1.05fr_0.95fr] lg:items-stretch">
        <AuthTerminal />
        <div className="ledger-panel" aria-label="Recent decisions, tamper-evident log">
          <div className="ledger-head">
            <span>Recent decisions · hash-chained</span>
            <span className="inline-flex items-center gap-2">
              <span className="live-dot" aria-hidden="true" />
              live
            </span>
          </div>
          {LEDGER.map((r) => (
            <div key={r.hash} className="ledger-row">
              <span className="time">{r.time}</span>
              <span className="ledger-verdict" style={{ color: VERDICT_COLOR[r.verdict] }}>
                <span
                  className="dot"
                  style={{ background: VERDICT_COLOR[r.verdict] }}
                  aria-hidden="true"
                />
                {r.label}
              </span>
              <span className="merchant">{r.merchant}</span>
              <span className="amount">{r.amount}</span>
              <span className="hash">{r.hash}</span>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}

function AuthTerminal() {
  return (
    <div
      className="terminal"
      role="img"
      aria-label="A $4,820 charge to an unknown merchant is blocked by TrustLoopGuard for exceeding the $5,000 per-day spend cap, with trace id tr_9f3a71c4 and a signed record."
    >
      <div className="terminal-bar">
        <span className="flex items-center gap-1.5" aria-hidden="true">
          <span className="dot" style={{ background: 'var(--color-block)' }} />
          <span className="dot" style={{ background: 'var(--color-rewrite)' }} />
          <span className="dot" style={{ background: 'var(--color-allow)' }} />
        </span>
        <span>POST /v1/events</span>
        <span>agent · payments-ap</span>
      </div>

      <p className="terminal-label">Proposed action</p>
      <dl>
        <div className="row">
          <dt className="k">type</dt>
          <dd className="v">charge</dd>
        </div>
        <div className="row">
          <dt className="k">merchant</dt>
          <dd className="v">Northwind Supplies Co.</dd>
        </div>
        <div className="row">
          <dt className="k">amount</dt>
          <dd className="v amount">$4,820.00 USD</dd>
        </div>
      </dl>

      <div className="cc-wrap">
        <p className="cc-label">card on file</p>
        <div className="cc">
          <div className="cc-shine" aria-hidden="true" />
          <div className="cc-top">
            <span className="cc-chip" aria-hidden="true" />
            <span className="cc-net">TLG&nbsp;PAY</span>
          </div>
          <div className="cc-number">···· ···· ···· 4242</div>
          <div className="cc-bot">
            <span className="cc-field">
              <span className="cc-cap">card holder</span>
              ACME PAYMENTS
            </span>
            <span className="cc-field">
              <span className="cc-cap">exp</span>
              10 / 27
            </span>
          </div>
        </div>
      </div>

      <div className="checkpoint">
        <span>▸ trustloopguard.check()</span>
        <span>policy · payments/daily-cap</span>
      </div>

      <div className="verdict-block">
        <div className="verdict-head">
          <span className="verdict-name">BLOCK</span>
          <span className="stamp" aria-hidden="true">
            <svg
              width="11"
              height="11"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2.5"
            >
              <circle cx="12" cy="12" r="9" />
              <path d="M6 6l12 12" strokeLinecap="round" />
            </svg>
            declined
          </span>
        </div>
        <dl className="verdict-meta">
          <dt className="k">reason</dt>
          <dd className="v">exceeds daily spend cap by $3,730</dd>
          <dt className="k">cap</dt>
          <dd className="v">$5,000.00 / day · $3,910.00 already spent</dd>
          <dt className="k">trace</dt>
          <dd className="v">tr_9f3a71c4e2</dd>
          <dt className="k">signed</dt>
          <dd className="v signed">✓ ed25519 · 14:32:07Z</dd>
        </dl>
      </div>
    </div>
  );
}
