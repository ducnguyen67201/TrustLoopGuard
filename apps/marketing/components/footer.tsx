import { GITHUB_URL } from '@/lib/github';

const LINKS = {
  product: [
    { href: '#how', label: 'How it works' },
    { href: '#verdicts', label: 'Verdicts' },
    { href: '#sdk', label: 'SDK' },
    { href: '#why', label: 'Why' },
  ],
  developers: [
    { href: '/docs', label: 'Documentation' },
    { href: GITHUB_URL, label: 'GitHub' },
    {
      href: `${GITHUB_URL}/blob/main/docs/SDK_DRIVEN.md`,
      label: 'Developer guide',
    },
  ],
} as const;

export function Footer() {
  return (
    <footer>
      <div className="mx-auto max-w-6xl px-6 py-16">
        <div className="grid gap-12 sm:grid-cols-2 lg:grid-cols-4">
          <div>
            <div className="flex items-center gap-2 text-sm font-medium">
              <Logo />
              TrustLoopGuard
            </div>
            <p className="mt-4 max-w-xs text-sm leading-relaxed text-[var(--color-ink-dim)]">
              Real-time guardrails for AI agents. Verdicts in milliseconds.
            </p>
          </div>

          <Column title="Product" links={LINKS.product} />
          <Column title="Developers" links={LINKS.developers} />

          <div>
            <div className="text-xs font-medium uppercase tracking-[0.16em] text-[var(--color-ink-mute)]">
              Project
            </div>
            <div className="mt-4 inline-flex items-center gap-2 text-xs text-[var(--color-ink-dim)]">
              <span className="pulse-dot inline-block h-1.5 w-1.5 rounded-full bg-[var(--color-accent)]" />
              Open source · actively developed
            </div>
          </div>
        </div>

        <div className="mt-16 flex flex-col gap-3 border-t border-[var(--color-border)] pt-8 text-xs text-[var(--color-ink-mute)] sm:flex-row sm:items-center sm:justify-between">
          <span>© {new Date().getFullYear()} TrustLoopGuard contributors</span>
          <span>Built for teams shipping AI agents.</span>
        </div>
      </div>
    </footer>
  );
}

function Logo() {
  return (
    <svg
      width="18"
      height="18"
      viewBox="0 0 20 20"
      fill="none"
      aria-hidden
    >
      <rect
        x="1"
        y="1"
        width="18"
        height="18"
        rx="5"
        fill="var(--color-ink)"
      />
      <path
        d="M6 10.5L9 13L14 7.5"
        stroke="white"
        strokeWidth="1.75"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

function Column({
  title,
  links,
}: {
  title: string;
  links: readonly { href: string; label: string }[];
}) {
  return (
    <div>
      <div className="text-xs font-medium uppercase tracking-[0.16em] text-[var(--color-ink-mute)]">
        {title}
      </div>
      <ul className="mt-4 space-y-2.5 text-sm">
        {links.map((l) => (
          <li key={l.href}>
            <a href={l.href} className="link-quiet">
              {l.label}
            </a>
          </li>
        ))}
      </ul>
    </div>
  );
}
