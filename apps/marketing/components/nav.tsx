import Link from 'next/link';
import { GITHUB_URL, formatStars, getStarCount } from '@/lib/github';

const NAV_LINKS = [
  { href: '#how', label: 'How it works' },
  { href: '#verdicts', label: 'Verdicts' },
  { href: '#sdk', label: 'SDK' },
  { href: '#why', label: 'Why' },
] as const;

export async function Nav() {
  const stars = await getStarCount();

  return (
    <header className="fixed top-0 inset-x-0 z-40">
      <nav
        aria-label="Main navigation"
        className="mx-auto max-w-6xl px-6 pt-5"
      >
        <div className="glass flex items-center justify-between rounded-full px-4 py-2.5">
          <Link
            href="/"
            className="flex items-center gap-2 px-2 text-sm font-medium tracking-tight"
          >
            <span
              aria-hidden
              className="grid h-6 w-6 place-items-center rounded-full bg-[var(--color-accent)] text-white text-[10px] font-bold"
            >
              T
            </span>
            <span>TrustLoopGuard</span>
          </Link>
          <ul className="hidden md:flex items-center gap-1 text-sm text-[var(--color-ink-dim)]">
            {NAV_LINKS.map((link) => (
              <li key={link.href}>
                <a
                  href={link.href}
                  className="rounded-full px-3 py-1.5 hover:text-[var(--color-ink)] hover:bg-[var(--color-accent-soft)] transition-colors"
                >
                  {link.label}
                </a>
              </li>
            ))}
          </ul>
          <div className="flex items-center gap-2">
            <GitHubStarLink stars={stars} />
            <a
              href="#quickstart"
              className="inline-flex items-center gap-1.5 rounded-full bg-[var(--color-ink)] px-4 py-1.5 text-sm font-medium text-white hover:bg-[var(--color-accent)] transition-colors"
            >
              Quickstart
            </a>
          </div>
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
          : `View TrustLoopGuard on GitHub — ${stars} stars`
      }
      className="hidden sm:inline-flex items-center gap-2 rounded-full glass-tight px-3 py-1.5 text-sm font-medium text-[var(--color-ink-dim)] hover:text-[var(--color-ink)] hover:bg-white/80 transition-colors"
    >
      <GitHubMark />
      <span>GitHub</span>
      {stars !== null && (
        <span className="flex items-center gap-1 border-l border-[var(--color-hairline)] pl-2 text-[var(--color-ink)]">
          <StarIcon />
          <span className="tabular-nums">{formatStars(stars)}</span>
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
