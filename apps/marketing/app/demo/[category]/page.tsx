import type { Metadata } from 'next';
import { notFound } from 'next/navigation';

import { getGenericDemoProfile } from '../../../lib/server/outbound-demo-profile-store';
import { CONTEXTUAL_PAGE_COPY } from '../contextual-content';
import { PersonalizedContextualDemoPageContent } from '../personalized-contextual-page';

type CompanyDemoPageProps = {
  params: Promise<{ category: string }>;
};

export const dynamic = 'force-dynamic';

export async function generateMetadata({ params }: CompanyDemoPageProps): Promise<Metadata> {
  const { category } = await params;
  const profile = await getGenericDemoProfile(category);
  const copy = CONTEXTUAL_PAGE_COPY.en;

  return {
    title: profile ? copy.title(profile.company_name) : copy.fallbackTitle,
    description: profile
      ? copy.description(profile.company_name)
      : copy.fallbackDescription,
    alternates: { canonical: profile?.demo_url ?? '/demo' },
    robots: { index: false, follow: false },
  };
}

export default async function CompanyDemoPage({ params }: CompanyDemoPageProps) {
  const { category } = await params;
  const profile = await getGenericDemoProfile(category);
  if (!profile) {
    notFound();
  }

  return (
    <PersonalizedContextualDemoPageContent
      profile={profile}
      locale="en"
      pagePath={`/demo/${profile.slug}`}
    />
  );
}
