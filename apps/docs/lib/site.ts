// Single source of truth for the public origin used in agent-facing
// endpoints (llms.txt, llms-full.txt, robots.txt). Set NEXT_PUBLIC_SITE_URL
// at deploy time; falls back to the local dev port so endpoints work
// out of the box.
export const SITE_URL =
  process.env['NEXT_PUBLIC_SITE_URL']?.replace(/\/$/, '') ?? 'http://localhost:3001';
