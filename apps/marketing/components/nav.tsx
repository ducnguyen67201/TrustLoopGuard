import Link from 'next/link';
import { BOOK_MEETING_URL, GITHUB_URL, formatStars, getStarCount } from '@/lib/github';

const NAV_LINKS = [
  { href: '#demo', label: 'Demo' },
  { href: '#problem', label: 'Problem' },
  { href: '#loop', label: 'Runtime loop' },
  { href: '#live', label: 'Live' },
  { href: '#quickstart', label: 'Quickstart' },
  { href: '#monitoring', label: 'Monitoring' },
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
          <span>TrustLoopGuard</span>
        </Link>
        <ul className="hidden items-center gap-6 text-sm text-[var(--color-muted)] lg:flex">
          {NAV_LINKS.map((link) => (
            <li key={link.href}>
              <a href={link.href} className="transition-colors hover:text-[var(--color-ink)]">
                {link.label}
              </a>
            </li>
          ))}
        </ul>
        <div className="flex items-center gap-2">
          <GitHubStarLink stars={stars} />
          <a
            href={BOOK_MEETING_URL}
            target="_blank"
            rel="noreferrer"
            className="button-accent h-9 px-3 text-sm sm:px-4"
          >
            <span className="min-[360px]:hidden">Book call</span>
            <span className="hidden min-[360px]:inline">Book a meeting</span>
          </a>
        </div>
      </nav>
    </header>
  );
}

function GitHubStarLink({ stars }: { stars: number | null }) {
  return (
    <a
      href={GITHUB_URL}
      target="_blank"
      rel="noreferrer"
      aria-label={
        stars === null
          ? 'View TrustLoopGuard on GitHub'
          : `View TrustLoopGuard on GitHub - ${stars} stars`
      }
      className="hidden h-9 items-center gap-2 rounded-sm border border-[var(--color-line)] px-3 text-sm font-medium text-[var(--color-muted)] transition-colors hover:border-[var(--color-line-strong)] hover:text-[var(--color-ink)] sm:inline-flex"
    >
      <GitHubMark />
      <span>GitHub</span>
      {stars !== null && (
        <span className="flex items-center gap-1 border-l border-[var(--color-line)] pl-2 text-[var(--color-ink)]">
          <StarIcon />
          <span className="tabular-nums">{formatStars(stars)}</span>
        </span>
      )}
    </a>
  );
}

function GitHubMark() {
  return (
    <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor" aria-hidden>
      <path d="M8 0C3.58 0 0 3.58 0 8a8 8 0 005.47 7.59c.4.07.55-.17.55-.38v-1.34c-2.23.48-2.7-1.07-2.7-1.07-.36-.92-.89-1.16-.89-1.16-.73-.5.05-.49.05-.49.8.06 1.23.83 1.23.83.72 1.22 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.22 2.2.82a7.6 7.6 0 014 0c1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.28.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.74.54 1.48v2.2c0 .21.15.46.55.38A8 8 0 0016 8c0-4.42-3.58-8-8-8z" />
    </svg>
  );
}

function StarIcon() {
  return (
    <svg
      width="12"
      height="12"
      viewBox="0 0 12 12"
      fill="currentColor"
      aria-hidden
      className="text-[var(--color-rewrite)]"
    >
      <path d="M6 0.5l1.7 3.45 3.8.55-2.75 2.68.65 3.8L6 9.18 2.6 10.98l.65-3.8L0.5 4.5l3.8-.55L6 .5z" />
    </svg>
  );
}
