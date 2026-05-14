const LINKS = {
  product: [
    { href: '#how', label: 'How it works' },
    { href: '#sdk', label: 'SDK' },
    { href: '#why', label: 'Why' },
  ],
  developers: [
    { href: '/docs', label: 'Documentation' },
    {
      href: 'https://github.com/ducnguyen67201/TrustLoopGuard',
      label: 'GitHub',
    },
    {
      href: 'https://github.com/ducnguyen67201/TrustLoopGuard/blob/main/docs/SDK_DRIVEN.md',
      label: 'Developer guide',
    },
  ],
} as const;

export function Footer() {
  return (
    <footer className="border-t border-[var(--color-hairline)] mt-12">
      <div className="mx-auto max-w-6xl px-6 py-16">
        <div className="grid gap-12 sm:grid-cols-2 lg:grid-cols-4">
          <div>
            <div className="flex items-center gap-2 text-sm font-medium">
              <span
                aria-hidden
                className="grid h-6 w-6 place-items-center rounded-full bg-[var(--color-accent)] text-white text-[10px] font-bold"
              >
                T
              </span>
              TrustLoopGuard
            </div>
            <p className="mt-4 max-w-xs text-sm leading-relaxed text-[var(--color-ink-dim)]">
              Real-time guardrail runtime for AI agents. Decisions in
              milliseconds.
            </p>
          </div>

          <Column title="Product" links={LINKS.product} />
          <Column title="Developers" links={LINKS.developers} />

          <div>
            <div className="text-xs uppercase tracking-[0.2em] text-[var(--color-ink-mute)]">
              Project
            </div>
            <div className="mt-4 inline-flex items-center gap-2 rounded-full glass-tight px-3 py-1.5 text-xs">
              <span className="pulse-dot inline-block h-1.5 w-1.5 rounded-full bg-[var(--color-accent)]" />
              Open source · actively developed
            </div>
          </div>
        </div>

        <div className="mt-16 flex flex-col gap-4 border-t border-[var(--color-hairline)] pt-8 text-xs text-[var(--color-ink-mute)] sm:flex-row sm:items-center sm:justify-between">
          <span>© {new Date().getFullYear()} TrustLoopGuard contributors</span>
          <span>Built for teams shipping AI agents.</span>
        </div>
      </div>
    </footer>
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
      <div className="text-xs uppercase tracking-[0.2em] text-[var(--color-ink-mute)]">
        {title}
      </div>
      <ul className="mt-4 space-y-2 text-sm">
        {links.map((l) => (
          <li key={l.href}>
            <a
              href={l.href}
              className="text-[var(--color-ink-dim)] hover:text-[var(--color-ink)] transition-colors"
            >
              {l.label}
            </a>
          </li>
        ))}
      </ul>
    </div>
  );
}
