import { BOOK_MEETING_URL, GITHUB_URL } from '@/lib/github';

const VERDICTS = ['allow', 'rewrite', 'block', 'escalate'] as const;

export function Hero() {
  return (
    <section className="hero-shell border-b border-[var(--color-line)] px-5 py-10">
      <div className="mx-auto max-w-6xl">
        <div className="hero-panel relative overflow-hidden border border-[var(--color-line)] bg-[var(--color-page)] px-6 pb-20 pt-24 text-center md:px-12 md:pb-24 md:pt-28">
          <div className="hero-audit-rail" aria-hidden="true">
            <span>POST /v1/check</span>
            <span>policy boundary</span>
            <span>trace required</span>
          </div>
          <div className="hero-animated-grid" aria-hidden="true" />
          <img src="/trustloop-logo.svg" alt="" aria-hidden="true" className="hero-watermark" />
          <div className="hero-policy-stamp" aria-hidden="true">
            policy.check
          </div>
          <div className="hero-route hero-route-a" aria-hidden="true" />
          <div className="hero-route hero-route-b" aria-hidden="true" />
          <div className="hero-glass-chip hero-glass-early" aria-hidden="true">
            <span>boundary</span>
            <strong>SDK / proxy</strong>
          </div>
          <div className="hero-glass-chip hero-glass-trace" aria-hidden="true">
            <span>decision.trace_id</span>
            <strong>audit ready</strong>
          </div>
          <div className="hero-glass-chip hero-glass-verdict" aria-hidden="true">
            <span>verdict</span>
            <strong>block / rewrite</strong>
          </div>
          <div className="hero-signal hero-signal-a" aria-hidden="true" />
          <div className="hero-signal hero-signal-b" aria-hidden="true" />
          <span className="hero-corner left-3 top-3" aria-hidden="true" />
          <span className="hero-corner right-3 top-3" aria-hidden="true" />
          <span className="hero-corner bottom-3 left-3" aria-hidden="true" />
          <span className="hero-corner bottom-3 right-3" aria-hidden="true" />

          <div className="relative z-10">
            <p className="eyebrow">Compliance boundary for production agents</p>
            <h1 className="mx-auto mt-6 max-w-5xl text-5xl font-semibold leading-[0.96] text-[var(--color-ink)] sm:text-7xl lg:text-[5.6rem]">
              Stop unsafe AI actions before users see them.
            </h1>
            <p className="mx-auto mt-6 max-w-3xl text-lg leading-8 text-[var(--color-muted)] sm:text-xl">
              Catch leaked data, bad replies, and risky tool calls in production. Use the SDK or
              proxy to return allow, rewrite, block, or escalate with a trace.
            </p>

            <div className="mt-10 flex flex-col items-center justify-center gap-3 sm:flex-row">
              <a href="#quickstart" className="button-primary h-12 px-6">
                Install the SDK
              </a>
              <a
                href={BOOK_MEETING_URL}
                target="_blank"
                rel="noreferrer"
                className="button-accent h-12 px-6"
              >
                Book a meeting
              </a>
              <a href={GITHUB_URL} className="button-secondary h-12 px-6">
                View GitHub
              </a>
            </div>
          </div>
        </div>

        <HeroVisual />

        <div className="mx-auto mt-5 max-w-2xl border border-[var(--color-line)] bg-white">
          <div className="flex items-center justify-between border-b border-[var(--color-line)] px-4 py-2 text-sm text-[var(--color-muted)]">
            <span>Quick install</span>
            <span>TypeScript</span>
          </div>
          <pre className="overflow-x-auto px-4 py-4 font-mono text-sm leading-7">
            <code>
              <span className="text-[var(--color-muted)]">$ </span>
              <span>npm install @trustloopguard/sdk</span>
            </code>
          </pre>
        </div>
      </div>
    </section>
  );
}

function HeroVisual() {
  return (
    <div className="hero-visual mt-5 overflow-hidden border border-[var(--color-line)] bg-white">
      <div className="hero-visual-map" aria-hidden="true" />
      <div className="relative grid gap-6 p-5 lg:grid-cols-[0.92fr_1.08fr_0.92fr] lg:p-7">
        <div className="grid gap-3">
          <VisualNode
            eyebrow="source"
            title="Agent app"
            body="Drafts, tool calls, workflow actions"
          />
          <VisualNode eyebrow="path A" title="SDK" body="Inline check() in your agent loop" />
          <VisualNode
            eyebrow="path B"
            title="Proxy server"
            body="Policy boundary in front of agent traffic"
            accent
          />
        </div>

        <div className="hero-core">
          <div className="flex items-center justify-center gap-3">
            <img
              src="/trustloop-logo.svg"
              alt=""
              aria-hidden="true"
              className="logo-mark h-10 w-10"
            />
            <div className="text-left">
              <p className="font-mono text-xs text-[var(--color-muted)]">TrustLoopGuard</p>
              <h2 className="text-2xl font-semibold">Runtime decision layer</h2>
            </div>
          </div>
          <div className="mt-7 grid grid-cols-2 gap-2">
            {VERDICTS.map((verdict) => (
              <span key={verdict} className={`verdict-chip verdict-chip-${verdict}`}>
                {verdict}
              </span>
            ))}
          </div>
          <div className="mt-7 rounded-sm border border-[var(--color-line)] bg-white p-4 text-left">
            <p className="font-mono text-xs text-[var(--color-muted)]">POST /v1/check</p>
            <p className="mt-2 text-sm leading-6">
              Return a verdict, safe rewrite, reason, and trace ID before delivery.
            </p>
          </div>
        </div>

        <div className="grid gap-3">
          <VisualNode eyebrow="trace" title="tr_7f3a" body="policy=support/private-data" />
          <VisualNode
            eyebrow="decision"
            title="rewrite"
            body="Remove private customer fields"
            accent
          />
          <VisualNode
            eyebrow="app action"
            title="send safe response"
            body="Your product still owns delivery"
          />
        </div>
      </div>
    </div>
  );
}

function VisualNode({
  eyebrow,
  title,
  body,
  accent = false,
}: {
  eyebrow: string;
  title: string;
  body: string;
  accent?: boolean;
}) {
  return (
    <article className={accent ? 'visual-node visual-node-accent' : 'visual-node'}>
      <p className="font-mono text-xs text-[var(--color-muted)]">{eyebrow}</p>
      <h2 className="mt-2 text-lg font-semibold">{title}</h2>
      <p className="mt-2 text-sm leading-6 text-[var(--color-muted)]">{body}</p>
    </article>
  );
}
