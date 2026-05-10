import { source } from '@/lib/source';
import { SITE_URL } from '@/lib/site';

export const dynamic = 'force-static';
export const revalidate = false;

// Single-fetch dump of the entire docs site as plain markdown. Each page
// is wrapped with a small header so an agent can attribute content to a
// specific URL.

export async function GET() {
  const pages = source.getPages();
  const sections = await Promise.all(
    pages.map(async (page) => {
      const md = await page.data.getText('processed');
      return [
        `# ${page.data.title}`,
        `URL: ${SITE_URL}${page.url}`,
        page.data.description ?? '',
        md,
      ]
        .filter(Boolean)
        .join('\n\n');
    }),
  );

  return new Response(sections.join('\n\n---\n\n') + '\n', {
    headers: {
      'content-type': 'text/plain; charset=utf-8',
      'cache-control': 'public, max-age=0, s-maxage=3600, stale-while-revalidate=86400',
    },
  });
}
