export const GITHUB_REPO = 'ducnguyen67201/TrustLoopGuard';
export const GITHUB_URL = `https://github.com/${GITHUB_REPO}`;
export const BOOK_MEETING_URL = 'https://calendar.app.google/aQc6ws3pDWpUKFzS9';

interface RepoSummary {
  stargazers_count: number;
}

/**
 * Fetches the current stargazer count from GitHub.
 * Cached for 5 minutes via Next's data cache so we don't hammer the API
 * on every render but the badge still tracks reality within ~5 min.
 */
export async function getStarCount(): Promise<number | null> {
  try {
    const res = await fetch(`https://api.github.com/repos/${GITHUB_REPO}`, {
      headers: {
        accept: 'application/vnd.github+json',
        'user-agent': 'trustloopguard-marketing',
      },
      next: { revalidate: 300 },
    });
    if (!res.ok) return null;
    const data = (await res.json()) as RepoSummary;
    return typeof data.stargazers_count === 'number' ? data.stargazers_count : null;
  } catch {
    return null;
  }
}

export function formatStars(n: number): string {
  if (n < 1000) return n.toString();
  if (n < 10_000) return `${(n / 1000).toFixed(1)}k`;
  if (n < 1_000_000) return `${Math.round(n / 1000)}k`;
  return `${(n / 1_000_000).toFixed(1)}M`;
}
