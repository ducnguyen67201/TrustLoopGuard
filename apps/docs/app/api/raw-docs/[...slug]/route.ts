import { getRawDocBySlug, textResponse } from '@/lib/llm-docs';

type RouteContext = {
  params: Promise<{
    slug?: string[];
  }>;
};

export async function GET(_request: Request, context: RouteContext) {
  const { slug } = await context.params;
  const page = await getRawDocBySlug(slug);
  if (!page) {
    return textResponse('Not found\n', { status: 404 });
  }

  return textResponse(page.raw);
}
