import { Eyebrow } from './how';
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
      className="relative mx-auto max-w-6xl px-6 py-32"
    >
      <div className="grid gap-12 lg:grid-cols-[1fr_1.4fr] lg:gap-20 lg:items-start">
        <div className="lg:sticky lg:top-32">
          <Eyebrow>Built for your stack</Eyebrow>
          <h2
            id="sdk-heading"
            className="mt-4 text-balance font-medium leading-[1.04] tracking-[-0.03em]"
            style={{ fontSize: 'var(--text-display)' }}
          >
            Three SDKs.{' '}
            <span className="text-[var(--color-ink-dim)]">
              Identical behavior in each.
            </span>
          </h2>
          <p className="mt-6 max-w-md text-[var(--color-ink-dim)] leading-relaxed">
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
    <div>
      <dt
        className="font-medium tracking-tight text-[var(--color-accent)]"
        style={{ fontSize: 'clamp(1.5rem, 0.8rem + 1.2vw, 2rem)' }}
      >
        {k}
      </dt>
      <dd className="mt-1 text-xs uppercase tracking-[0.16em] text-[var(--color-ink-mute)]">
        {v}
      </dd>
    </div>
  );
}
