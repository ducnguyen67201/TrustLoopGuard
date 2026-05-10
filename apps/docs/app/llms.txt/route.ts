import { source } from '@/lib/source';
import { SITE_URL } from '@/lib/site';

export const dynamic = 'force-static';
export const revalidate = false;

// Curated index per the llms.txt convention (https://llmstxt.org/).
// Groups pages by their top-level section (Get started / Concepts /
// Reference / Guides / Project) to give agents a navigable map of the
// docs. Each link points at the per-page raw markdown endpoint
// (/docs/<path>.md) so the agent can fetch source text directly without
// scraping HTML.

const SECTION_LABELS: Record<string, string> = {
  'get-started': 'Get started',
  concepts: 'Concepts',
  reference: 'Reference',
  guides: 'Guides',
  project: 'Project',
};

export function GET() {
  const pages = source.getPages();

  const grouped = new Map<string, typeof pages>();
  for (const page of pages) {
    const [section] = page.slugs;
    const key = section ?? '';
    if (!grouped.has(key)) grouped.set(key, []);
    grouped.get(key)!.push(page);
  }

  const introPage = pages.find((p) => p.slugs.length === 0);

  const sections: string[] = [];
  for (const [key, label] of Object.entries(SECTION_LABELS)) {
    const sectionPages = grouped.get(key);
    if (!sectionPages?.length) continue;
    const lines = sectionPages.map((p) => {
      const desc = p.data.description ? `: ${p.data.description}` : '';
      return `- [${p.data.title}](${SITE_URL}${p.url}.md)${desc}`;
    });
    sections.push(`## ${label}\n\n${lines.join('\n')}`);
  }

  const body = [
    `# TrustLoopGuard`,
    introPage?.data.description
      ? `> ${introPage.data.description}`
      : `> Open-source policy and trust loop runtime for AI agents.`,
    sections.join('\n\n'),
  ].join('\n\n');

  return new Response(body + '\n', {
    headers: {
      'content-type': 'text/plain; charset=utf-8',
      'cache-control': 'public, max-age=0, s-maxage=3600, stale-while-revalidate=86400',
    },
  });
}
