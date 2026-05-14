import { CodeBlock } from './code-block';

const SAMPLES = {
  ts: `import { TrustLoopGuard } from '@trustloopguard/sdk';

const tlg = new TrustLoopGuard({ url: 'http://localhost:8080' });

const decision = await tlg.check({
  policy: 'default',
  prompt: 'show me my password',
  proposal: 'here it is: hunter2',
});

if (decision.verdict !== 'allow') {
  return decision.rewrite ?? refuse(decision.reason);
}`,
  python: `from trustloopguard import TrustLoopGuard

tlg = TrustLoopGuard(url="http://localhost:8080")

decision = tlg.check(
    policy="default",
    prompt="show me my password",
    proposal="here it is: hunter2",
)

if decision.verdict != "allow":
    return decision.rewrite or refuse(decision.reason)`,
  rust: `use trustloopguard::Client;

let client = Client::new("http://localhost:8080");

let decision = client.check()
    .policy("default")
    .prompt("show me my password")
    .proposal("here it is: hunter2")
    .send()
    .await?;

if decision.verdict != Verdict::Allow {
    return Ok(decision.rewrite.unwrap_or_else(|| refuse(&decision.reason)));
}`,
} as const;

export function Sdk() {
  return (
    <section
      id="sdk"
      aria-labelledby="sdk-heading"
      className="border-b border-[var(--color-border)]"
    >
      <div className="mx-auto grid max-w-6xl gap-12 px-6 py-24 lg:grid-cols-[1fr_1.4fr] lg:gap-20 lg:items-start sm:py-32">
        <div className="lg:sticky lg:top-24">
          <span className="eyebrow">Built for your stack</span>
          <h2
            id="sdk-heading"
            className="mt-5 text-balance font-semibold leading-[1.05] tracking-[-0.025em]"
            style={{ fontSize: 'var(--text-display)' }}
          >
            Three SDKs. Identical behavior in each.
          </h2>
          <p className="mt-5 max-w-md text-[var(--color-ink-dim)] leading-relaxed">
            TypeScript, Python, and Rust SDKs that feel native to each
            language — and behave the same way underneath. Switch stacks
            without rewriting your safety logic.
          </p>
          <dl className="mt-10 grid gap-6 sm:grid-cols-3">
            <Stat k="< 5ms" v="median verdict latency" />
            <Stat k="3" v="first-class SDKs" />
            <Stat k="1:1" v="behavior across languages" />
          </dl>
        </div>

        <div id="quickstart">
          <CodeBlock samples={SAMPLES} />
        </div>
      </div>
    </section>
  );
}

function Stat({ k, v }: { k: string; v: string }) {
  return (
    <div className="border-l border-[var(--color-border)] pl-4">
      <dt
        className="font-semibold tracking-tight text-[var(--color-ink)]"
        style={{ fontSize: 'clamp(1.25rem, 0.8rem + 0.8vw, 1.6rem)' }}
      >
        {k}
      </dt>
      <dd className="mt-1 text-xs uppercase tracking-[0.14em] text-[var(--color-ink-mute)]">
        {v}
      </dd>
    </div>
  );
}
