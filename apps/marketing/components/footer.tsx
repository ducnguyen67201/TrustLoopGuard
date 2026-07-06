import { DOCS_URL, GITHUB_URL } from '@/lib/github';
import { landingPages } from '@/lib/seo';
import { Ascii } from './ascii-art';

const LINKS = [
  { href: '#problem', label: 'Problem' },
  { href: '#loop', label: 'Runtime loop' },
  { href: '#live', label: 'Verdicts' },
  { href: '#quickstart', label: 'Quickstart' },
  { href: '#updates', label: 'Stay in the loop' },
  { href: DOCS_URL, label: 'Docs' },
  { href: GITHUB_URL, label: 'GitHub' },
] as const;

const USE_CASE_LINKS = landingPages.map((page) => ({
  href: `/${page.slug}`,
  label: page.eyebrow,
}));

export function Footer() {
  return (
    <footer className="relative overflow-hidden border-t border-[var(--color-line)]">
      <Ascii
        name="padlock"
        className="ascii-sm ascii-faint ascii-drift absolute -bottom-2 right-8 hidden md:block"
      />
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
            Spend caps and an audit trail for AI agents that pay, refund, and move money.
          </p>
        </div>
        <nav aria-label="Footer navigation">
          <ul className="flex flex-wrap gap-x-5 gap-y-2 text-sm text-[var(--color-muted)]">
            {[...LINKS, ...USE_CASE_LINKS].map((link) => (
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
