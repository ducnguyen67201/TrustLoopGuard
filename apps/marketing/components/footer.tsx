import { GITHUB_URL } from '@/lib/github';

const LINKS = [
  { href: '#problem', label: 'Problem' },
  { href: '#loop', label: 'Runtime loop' },
  { href: '#verdicts', label: 'Verdicts' },
  { href: '#quickstart', label: 'Quickstart' },
  { href: '/docs', label: 'Docs' },
  { href: GITHUB_URL, label: 'GitHub' },
] as const;

export function Footer() {
  return (
    <footer className="border-t border-[var(--color-line)]">
      <div className="mx-auto flex max-w-6xl flex-col gap-8 px-5 py-10 md:flex-row md:items-center md:justify-between">
        <div>
          <div className="flex items-center gap-2 text-sm font-semibold">
            <img
              src="/trustloop-logo.svg"
              alt=""
              aria-hidden="true"
              className="logo-mark h-7 w-7"
            />
            TrustLoopGuard
          </div>
          <p className="mt-3 max-w-md text-sm leading-6 text-[var(--color-muted)]">
            Runtime guardrails for teams shipping AI agents.
          </p>
        </div>
        <nav aria-label="Footer navigation">
          <ul className="flex flex-wrap gap-x-5 gap-y-2 text-sm text-[var(--color-muted)]">
            {LINKS.map((link) => (
              <li key={link.href}>
                <a href={link.href} className="hover:text-[var(--color-ink)]">
                  {link.label}
                </a>
              </li>
            ))}
          </ul>
        </nav>
      </div>
    </footer>
  );
}
