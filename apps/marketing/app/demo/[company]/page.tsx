import type { Metadata } from 'next';
import { notFound } from 'next/navigation';

import { getActiveDemoProfileBySlug } from '../../../lib/server/outbound-demo-profile-store';
import { CompanyDemoPageContent } from '../company-demo-page';

type CompanyDemoPageProps = {
  params: Promise<{ company: string }>;
};

export const dynamic = 'force-dynamic';

export async function generateMetadata({ params }: CompanyDemoPageProps): Promise<Metadata> {
  const { company } = await params;
  const profile = await getActiveDemoProfileBySlug(company);

  return {
    title: profile ? `${profile.company_name} AI Guardrail Concept` : 'Personalized AI Guardrail Demo',
    description: profile
      ? `A private, public-source TrustLoopGuard concept for ${profile.company_name}.`
      : 'A private TrustLoopGuard concept demo.',
    alternates: { canonical: profile?.demo_url ?? '/demo' },
    robots: { index: false, follow: false },
  };
}

export default async function CompanyDemoPage({ params }: CompanyDemoPageProps) {
  const { company } = await params;
  const profile = await getActiveDemoProfileBySlug(company);
  if (!profile) {
    notFound();
  }

  return <CompanyDemoPageContent profile={profile} />;
}
