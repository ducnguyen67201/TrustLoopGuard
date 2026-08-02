import type { Metadata } from 'next';
import { notFound } from 'next/navigation';
import { UseCasePage } from '@/components/use-case-page';
import { absoluteUrl } from '@/lib/seo';
import { getUseCase, USE_CASES } from '../content';

interface UseCasePageProps {
  params: Promise<{ slug: string }>;
}

export function generateStaticParams() {
  return USE_CASES.map((useCase) => ({ slug: useCase.slug }));
}

export async function generateMetadata({ params }: UseCasePageProps): Promise<Metadata> {
  const { slug } = await params;
  const useCase = getUseCase(slug);
  if (!useCase) return {};

  const title = `${useCase.eyebrow} | Featherlane AI`;
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
  const { slug } = await params;
  const useCase = getUseCase(slug);
  if (!useCase) notFound();

  return <UseCasePage useCase={useCase} />;
}
