import { notFound } from 'next/navigation';
import {
  DocsPage,
  DocsBody,
  DocsTitle,
  DocsDescription,
  ViewOptionsPopover,
} from 'fumadocs-ui/layouts/docs/page';
import defaultMdxComponents from 'fumadocs-ui/mdx';
import { source } from '@/lib/source';
import { APIPage } from '@/lib/openapi';

export default async function Page(props: { params: Promise<{ slug?: string[] }> }) {
  const params = await props.params;
  const page = source.getPage(params.slug);
  if (!page) notFound();

  const MDX = page.data.body;

  // Per-page raw markdown URL: /docs/<slug>.md is rewritten to
  // /api/page-md/<slug> in next.config.mjs.
  const markdownUrl = page.url.endsWith('/') ? `${page.url.slice(0, -1)}.md` : `${page.url}.md`;
  // info.path is the file path relative to the content dir (e.g. concepts/architecture.mdx).
  const githubUrl = `https://github.com/ducnguyen67201/TrustLoopGuard/blob/main/apps/docs/content/docs/${page.data.info.path}`;

  return (
    <DocsPage toc={page.data.toc} full={page.data.full}>
      <DocsTitle>{page.data.title}</DocsTitle>
      <DocsDescription>{page.data.description}</DocsDescription>
      <DocsBody>
        <ViewOptionsPopover markdownUrl={markdownUrl} githubUrl={githubUrl} />
        <MDX components={{ ...defaultMdxComponents, APIPage }} />
      </DocsBody>
    </DocsPage>
  );
}

export function generateStaticParams() {
  return source.generateParams();
}

export async function generateMetadata(props: {
  params: Promise<{ slug?: string[] }>;
}) {
  const params = await props.params;
  const page = source.getPage(params.slug);
  if (!page) notFound();

  return {
    title: page.data.title,
    description: page.data.description,
  };
}
