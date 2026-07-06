import Link from 'next/link';
import { BOOK_MEETING_URL, GITHUB_URL, getStarCount } from '@/lib/github';
import { NavActions } from './nav-actions';

/* ponytail: three links max — the demo, the dev action, the mailing list.
   Everything else is one scroll away; the footer still links the rest. */
const NAV_LINKS = [
  { href: '/#live', label: 'Live' },
  { href: '/#quickstart', label: 'Quickstart' },
  { href: '/#updates', label: 'Updates', highlight: true },
] as const;

export async function Nav() {
  const stars = await getStarCount();

  return (
    <header className="sticky top-0 inset-x-0 z-40 border-b border-[var(--color-line)] bg-[var(--color-page)]/92 backdrop-blur">
      <nav
        aria-label="Main navigation"
        className="mx-auto flex h-16 max-w-6xl items-center justify-between px-5"
      >
        <Link href="/" className="flex items-center gap-2 text-sm font-semibold">
          <img src="/trustloop-logo.svg" alt="" aria-hidden="true" className="logo-mark h-7 w-7" />
          {/* mono wordmark is wide — icon only on the narrowest screens */}
          <span className="hidden min-[430px]:inline">TrustLoopGuard</span>
        </Link>
        <ul className="hidden items-center gap-6 whitespace-nowrap text-sm text-[var(--color-muted)] lg:flex">
          {NAV_LINKS.map((link) => (
            <li key={link.href}>
              <a
                href={link.href}
                className={
                  'highlight' in link && link.highlight
                    ? 'nav-highlight transition-colors'
                    : 'transition-colors hover:text-[var(--color-ink)]'
                }
              >
                {link.label}
              </a>
            </li>
          ))}
        </ul>
        <NavActions bookMeetingUrl={BOOK_MEETING_URL} githubUrl={GITHUB_URL} stars={stars} />
      </nav>
    </header>
  );
}
