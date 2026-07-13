import type { Metadata } from 'next';
import Link from 'next/link';
import { notFound } from 'next/navigation';
import { Footer } from '@/components/footer';
import { Nav } from '@/components/nav';
import { ScrollTopButton } from '@/components/scroll-top-button';
import { UseCaseDetail } from '@/components/use-case-detail';
import { absoluteUrl } from '@/lib/seo';
import { getUseCase, USE_CASES } from '../content';

interface UseCasePageProps {
  params: Promise<{ slug: string }>;
}

export function generateStaticParams() {
  return USE_CASES.map((useCase) => ({ slug: useCase.slug }));
}

export async function generateMetadata({ params }: UseCasePageProps): Promise<Metadata> {
  const useCase = getUseCase((await params).slug);
  if (!useCase) return {};

  const title = `${useCase.eyebrow} | TrustLoopGuard`;
  return {
    title: { absolute: title },
    description: useCase.summary,
    alternates: { canonical: useCase.href },
    openGraph: {
      title,
      description: useCase.summary,
      url: absoluteUrl(useCase.href),
      type: 'website',
    },
    twitter: {
      card: 'summary_large_image',
      title,
      description: useCase.summary,
    },
  };
}

export default async function Page({ params }: UseCasePageProps) {
  const useCase = getUseCase((await params).slug);
  if (!useCase) notFound();

  return (
    <>
      <Nav />
      <main>
        <div className="use-case-breadcrumb">
          <Link href="/use-cases">Use cases</Link>
          <span aria-hidden="true">/</span>
          <span>{useCase.eyebrow}</span>
        </div>
        <UseCaseDetail useCase={useCase} />
        <RelatedUseCases currentSlug={useCase.slug} />
      </main>
      <Footer />
      <ScrollTopButton />
    </>
  );
}

function RelatedUseCases({ currentSlug }: { currentSlug: string }) {
  const related = USE_CASES.filter((useCase) => useCase.slug !== currentSlug);

  return (
    <section className="section use-case-related" aria-labelledby="related-use-cases-heading">
      <p className="eyebrow">Other control points</p>
      <h2 id="related-use-cases-heading">Explore the other use cases.</h2>
      <div>
        {related.map((useCase) => (
          <Link key={useCase.slug} href={useCase.href}>
            <span>{useCase.number}</span>
            <strong>{useCase.eyebrow}</strong>
            <small>{useCase.title}</small>
            <i aria-hidden="true">→</i>
          </Link>
        ))}
      </div>
    </section>
  );
}
