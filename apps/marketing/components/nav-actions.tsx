'use client';

import { usePathname } from 'next/navigation';
import { MarketingEventLink } from './marketing-event-link';

interface NavActionsProps {
  bookMeetingUrl: string;
  githubUrl: string;
  stars: number | null;
}

export function NavActions({ bookMeetingUrl, githubUrl, stars }: NavActionsProps) {
  const page = usePathname() || '/';

  return (
    <div className="flex items-center gap-2">
      <GitHubStarLink githubUrl={githubUrl} page={page} stars={stars} />
      <MarketingEventLink
        href={bookMeetingUrl}
        target="_blank"
        className="button-primary h-10 px-4 text-sm"
        event="book_meeting_click"
        eventParams={{ page, location: 'nav', label: 'Book a meeting' }}
      >
        <span className="min-[420px]:hidden">Talk</span>
        <span className="hidden min-[420px]:inline">Talk to us</span>
      </MarketingEventLink>
    </div>
  );
}

function GitHubStarLink({
  githubUrl,
  page,
  stars,
}: {
  githubUrl: string;
  page: string;
  stars: number | null;
}) {
  return (
    <MarketingEventLink
      href={githubUrl}
      target="_blank"
      aria-label={
        stars === null
          ? 'View TrustLoopGuard on GitHub'
          : `View TrustLoopGuard on GitHub - ${stars} stars`
      }
      className="github-link hidden h-10 items-center gap-2 px-3 text-sm font-medium sm:inline-flex"
      event="github_click"
      eventParams={{ page, location: 'nav', label: 'GitHub' }}
    >
      <GitHubMark />
      <span>GitHub</span>
      {stars !== null && (
        <span className="flex items-center gap-1 border-l border-[var(--color-line)] pl-2 text-[var(--color-ink)]">
          <StarIcon />
          <span className="tabular-nums">{formatStars(stars)}</span>
        </span>
      )}
    </MarketingEventLink>
  );
}

function formatStars(n: number): string {
  if (n < 1000) return n.toString();
  if (n < 10_000) return `${(n / 1000).toFixed(1)}k`;
  if (n < 1_000_000) return `${Math.round(n / 1000)}k`;
  return `${(n / 1_000_000).toFixed(1)}M`;
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
      className="text-[var(--color-accent)]"
    >
      <path d="M6 0.5l1.7 3.45 3.8.55-2.75 2.68.65 3.8L6 9.18 2.6 10.98l.65-3.8L0.5 4.5l3.8-.55L6 .5z" />
    </svg>
  );
}
