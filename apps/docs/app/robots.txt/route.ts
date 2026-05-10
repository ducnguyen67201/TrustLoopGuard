import { SITE_URL } from '@/lib/site';

export const dynamic = 'force-static';
export const revalidate = false;

// Allow all crawlers (including AI training crawlers) and surface the
// llms.txt index so well-behaved agents can find the curated entry point
// without scraping HTML.
export function GET() {
  const body = [
    `User-agent: *`,
    `Allow: /`,
    ``,
    `# Curated index for LLM agents (https://llmstxt.org/)`,
    `Sitemap: ${SITE_URL}/llms.txt`,
    ``,
  ].join('\n');

  return new Response(body, {
    headers: { 'content-type': 'text/plain; charset=utf-8' },
  });
}
