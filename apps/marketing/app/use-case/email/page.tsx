import type { Metadata } from 'next';
import { EMAIL_USE_CASE } from '@/app/use-cases/content';
import { UseCasePage } from '@/components/use-case-page';
import { absoluteUrl } from '@/lib/seo';

const PAGE_TITLE = `${EMAIL_USE_CASE.eyebrow} | TrustLoopGuard`;

export const metadata: Metadata = {
  title: { absolute: PAGE_TITLE },
  description: EMAIL_USE_CASE.summary,
  alternates: { canonical: EMAIL_USE_CASE.href },
  openGraph: {
    title: PAGE_TITLE,
    description: EMAIL_USE_CASE.summary,
    url: absoluteUrl(EMAIL_USE_CASE.href),
    type: 'website',
  },
  twitter: {
    card: 'summary_large_image',
    title: PAGE_TITLE,
    description: EMAIL_USE_CASE.summary,
  },
};

export default function Page() {
  return <UseCasePage useCase={EMAIL_USE_CASE} />;
}
