import { Eyebrow } from './how';

const REASONS = [
  {
    title: 'Real-time, not after-the-fact',
    body: 'Safety checks run inline, before output reaches your customers. No nightly eval batches, no incident reviews after the fact.',
    icon: BoltIcon,
    span: 'lg:col-span-2',
  },
  {
    title: 'Every verdict is auditable',
    body: 'Each call returns the verdict, the reason, a trace ID, end-to-end latency, and the exact checks that fired.',
    icon: SignalIcon,
    span: '',
  },
  {
    title: 'Policies live in version control',
    body: 'Plain YAML. Diffable, reviewable, and shipped through the same pull-request flow as the rest of your code.',
    icon: YamlIcon,
    span: '',
  },
  {
    title: 'Consistent across your stack',
    body: 'TypeScript, Python, and Rust SDKs all enforce the same behavior. The verdict you see in staging is the verdict you ship in production.',
    icon: WireIcon,
    span: 'lg:col-span-2',
  },
] as const;

export function Why() {
  return (
    <section
      id="why"
      aria-labelledby="why-heading"
      className="relative mx-auto max-w-6xl px-6 py-32"
    >
      <Eyebrow>Why TrustLoopGuard</Eyebrow>
      <h2
        id="why-heading"
        className="mt-4 max-w-3xl text-balance font-medium leading-[1.04] tracking-[-0.03em]"
        style={{ fontSize: 'var(--text-display)' }}
      >
        Guardrails that ship{' '}
        <span className="text-[var(--color-ink-dim)]">
          alongside your agent, not after the fact.
        </span>
      </h2>

      <div className="mt-16 grid gap-4 lg:grid-cols-3">
        {REASONS.map(({ title, body, icon: Icon, span }) => (
          <article
            key={title}
            className={`glass group relative overflow-hidden rounded-2xl p-7 ${span}`}
          >
            <Icon />
            <h3 className="mt-6 text-xl font-medium tracking-tight">
              {title}
            </h3>
            <p className="mt-3 max-w-md text-sm leading-relaxed text-[var(--color-ink-dim)]">
              {body}
            </p>
          </article>
        ))}
      </div>
    </section>
  );
}

const ICON_CLS =
  'h-9 w-9 rounded-xl grid place-items-center bg-[var(--color-accent-soft)] text-[var(--color-accent-deep)]';

function BoltIcon() {
  return (
    <div className={ICON_CLS}>
      <svg
        width="18"
        height="18"
        viewBox="0 0 18 18"
        fill="none"
        aria-hidden
      >
        <path
          d="M10 1L2 10h6l-1 7 8-9h-6l1-7z"
          stroke="currentColor"
          strokeWidth="1.5"
          strokeLinejoin="round"
        />
      </svg>
    </div>
  );
}

function SignalIcon() {
  return (
    <div className={ICON_CLS}>
      <svg
        width="18"
        height="18"
        viewBox="0 0 18 18"
        fill="none"
        aria-hidden
      >
        <path
          d="M3 14V9M7 14V6M11 14V3M15 14v-3"
          stroke="currentColor"
          strokeWidth="1.5"
          strokeLinecap="round"
        />
      </svg>
    </div>
  );
}

function YamlIcon() {
  return (
    <div className={ICON_CLS}>
      <svg
        width="18"
        height="18"
        viewBox="0 0 18 18"
        fill="none"
        aria-hidden
      >
        <path
          d="M3 4h12M3 9h12M3 14h7"
          stroke="currentColor"
          strokeWidth="1.5"
          strokeLinecap="round"
        />
      </svg>
    </div>
  );
}

function WireIcon() {
  return (
    <div className={ICON_CLS}>
      <svg
        width="18"
        height="18"
        viewBox="0 0 18 18"
        fill="none"
        aria-hidden
      >
        <circle cx="4" cy="9" r="2" stroke="currentColor" strokeWidth="1.5" />
        <circle cx="14" cy="9" r="2" stroke="currentColor" strokeWidth="1.5" />
        <path d="M6 9h6" stroke="currentColor" strokeWidth="1.5" />
      </svg>
    </div>
  );
}
