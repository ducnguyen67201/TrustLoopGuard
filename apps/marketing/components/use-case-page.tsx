import Link from 'next/link';
import { USE_CASES, type UseCaseData } from '@/app/use-cases/content';
import { Footer } from './footer';
import { Nav } from './nav';
import { ScrollTopButton } from './scroll-top-button';
import { UseCaseDetail } from './use-case-detail';

export function UseCasePage({ useCase }: { useCase: UseCaseData }) {
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
