import { notFound } from 'next/navigation';
import { DocsPage, DocsBody, DocsTitle, DocsDescription } from 'fumadocs-ui/page';
import type { ComponentProps } from 'react';
import { APIPage } from '@/components/api-page';
import { mdxComponents } from '@/components/mdx';
import { openapi } from '@/lib/openapi';
import { source } from '@/lib/source';

export default async function Page(props: { params: Promise<{ slug?: string[] }> }) {
  const params = await props.params;
  const page = source.getPage(params.slug);
  if (!page) notFound();

  const MDX = page.data.body;
  const openapiPageProps = await openapi.preloadOpenAPIPage(page);
  const OpenAPIPage = (props: ComponentProps<typeof APIPage>) => (
    <APIPage {...props} {...openapiPageProps} />
  );

  return (
    <DocsPage toc={page.data.toc} full={page.data.full}>
      <DocsTitle>{page.data.title}</DocsTitle>
      <DocsDescription>{page.data.description}</DocsDescription>
      <DocsBody>
        <MDX
          components={{
            ...mdxComponents,
            APIPage: OpenAPIPage,
            OpenAPIPage,
          }}
        />
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
