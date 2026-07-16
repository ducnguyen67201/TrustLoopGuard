import { notFound, permanentRedirect } from 'next/navigation';
import { getUseCase, USE_CASES } from '@/app/use-cases/content';

interface LegacyUseCasePageProps {
  params: Promise<{ slug: string }>;
}

export function generateStaticParams() {
  return USE_CASES.map((useCase) => ({ slug: useCase.slug }));
}

export default async function Page({ params }: LegacyUseCasePageProps) {
  const useCase = getUseCase((await params).slug);
  if (!useCase) notFound();

  permanentRedirect(useCase.href);
}
