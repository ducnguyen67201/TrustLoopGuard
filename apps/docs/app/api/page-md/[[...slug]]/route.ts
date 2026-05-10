import { source } from '@/lib/source';

export const dynamic = 'force-static';
export const revalidate = false;

export function generateStaticParams() {
  return source.generateParams();
}

export async function GET(
  _req: Request,
  { params }: { params: Promise<{ slug?: string[] }> },
) {
  const { slug } = await params;
  const page = source.getPage(slug);
  if (!page) return new Response('Not Found', { status: 404 });

  const md = await page.data.getText('processed');
  return new Response(md, {
    headers: {
      'content-type': 'text/markdown; charset=utf-8',
      'cache-control': 'public, max-age=0, s-maxage=3600, stale-while-revalidate=86400',
    },
  });
}
