import type { Metadata } from 'next';
import { notFound } from 'next/navigation';

import { CONTEXTUAL_PAGE_COPY } from '@/app/demo/contextual-content';
import { PersonalizedContextualDemoPageContent } from '@/app/demo/personalized-contextual-page';
import { getGenericDemoProfile } from '@/lib/server/outbound-demo-profile-store';

type VietnameseCompanyDemoPageProps = {
  params: Promise<{ category: string }>;
};

export const dynamic = 'force-dynamic';

export async function generateMetadata({
  params,
}: VietnameseCompanyDemoPageProps): Promise<Metadata> {
  const { category } = await params;
  const profile = await getGenericDemoProfile(category);
  const copy = CONTEXTUAL_PAGE_COPY.vi;

  return {
    title: profile ? copy.title(profile.company_name) : copy.fallbackTitle,
    description: profile
      ? copy.description(profile.company_name)
      : copy.fallbackDescription,
    alternates: {
      canonical: profile ? `/vi/demo/${profile.slug}` : '/vi/demo/healthcare',
      languages: { 'vi-VN': profile ? `/vi/demo/${profile.slug}` : '/vi/demo/healthcare' },
    },
    robots: { index: false, follow: false },
    openGraph: { locale: 'vi_VN' },
  };
}

export default async function VietnameseCompanyDemoPage({
  params,
}: VietnameseCompanyDemoPageProps) {
  const { category } = await params;
  const profile = await getGenericDemoProfile(category);
  if (!profile) notFound();

  return (
    <PersonalizedContextualDemoPageContent
      profile={profile}
      locale="vi"
      pagePath={`/vi/demo/${profile.slug}`}
    />
  );
}
