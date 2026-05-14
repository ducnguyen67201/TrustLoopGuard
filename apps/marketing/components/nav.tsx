import Link from 'next/link';
import { GITHUB_URL, formatStars, getStarCount } from '@/lib/github';

const NAV_LINKS = [
  { href: '#how', label: 'How it works' },
  { href: '#verdicts', label: 'Verdicts' },
  { href: '#sdk', label: 'SDK' },
  { href: '/docs', label: 'Docs' },
] as const;

export async function Nav() {
  const stars = await getStarCount();

  return (
    <header className="sticky top-0 z-40 border-b border-[var(--color-border)] bg-[var(--color-canvas)]/85 backdrop-blur">
      <nav
        aria-label="Main navigation"
        className="mx-auto flex h-14 max-w-6xl items-center justify-between px-6"
      >
        <Link
          href="/"
          className="flex items-center gap-2 text-sm font-medium tracking-tight"
        >
          <Logo />
          <span>TrustLoopGuard</span>
        </Link>

        <ul className="hidden md:flex items-center gap-6 text-sm">
          {NAV_LINKS.map((link) => (
            <li key={link.href}>
              <a href={link.href} className="link-quiet">
                {link.label}
              </a>
            </li>
          ))}
        </ul>

        <div className="flex items-center gap-2">
          <GitHubStarLink stars={stars} />
          <a href="#quickstart" className="btn-primary">
            Get started
          </a>
        </div>
      </nav>
    </header>
  );
}

function Logo() {
  return (
    <svg
      width="20"
      height="20"
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

function GitHubStarLink({ stars }: { stars: number | null }) {
  return (
    <a
      href={GITHUB_URL}
      target="_blank"
      rel="noreferrer"
      aria-label={
        stars === null
          ? 'View TrustLoopGuard on GitHub'
          : `View TrustLoopGuard on GitHub — ${stars} stars`
      }
      className="hidden sm:inline-flex h-9 items-center gap-2 rounded-lg border border-[var(--color-border)] bg-[var(--color-surface)] px-3 text-sm font-medium text-[var(--color-ink-dim)] hover:border-[var(--color-border-strong)] hover:text-[var(--color-ink)] transition-colors"
    >
      <GitHubMark />
      <span>Star</span>
      {stars !== null && (
        <span className="border-l border-[var(--color-border)] pl-2 font-mono text-[12px] tabular-nums text-[var(--color-ink)]">
          {formatStars(stars)}
        </span>
      )}
    </a>
  );
}

function GitHubMark() {
  return (
    <svg
      width="14"
      height="14"
      viewBox="0 0 16 16"
      fill="currentColor"
      aria-hidden
    >
      <path d="M8 0C3.58 0 0 3.58 0 8a8 8 0 005.47 7.59c.4.07.55-.17.55-.38v-1.34c-2.23.48-2.7-1.07-2.7-1.07-.36-.92-.89-1.16-.89-1.16-.73-.5.05-.49.05-.49.8.06 1.23.83 1.23.83.72 1.22 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.22 2.2.82a7.6 7.6 0 014 0c1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.28.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.74.54 1.48v2.2c0 .21.15.46.55.38A8 8 0 0016 8c0-4.42-3.58-8-8-8z" />
    </svg>
  );
}
