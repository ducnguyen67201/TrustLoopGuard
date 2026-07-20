import type { Metadata } from 'next';
import { notFound } from 'next/navigation';

import { getActiveDemoProfile } from '../../../../lib/server/outbound-demo-profile-store';
import { CompanyDemoPageContent } from '../../company-demo-page';
import { demoScenarioIdByCategory } from '../../company-profile';

type PersonalizedResearchDemoPageProps = {
  params: Promise<{ company: string }>;
};

export const dynamic = 'force-dynamic';

export async function generateMetadata({
  params,
}: PersonalizedResearchDemoPageProps): Promise<Metadata> {
  const { company } = await params;
  const profile = await getResearchProfile(company);

  return {
    title: profile
      ? `${profile.company_name} Research Agent Concept`
      : 'Personalized Research Agent Demo',
    description: profile
      ? `A private, public-source TrustLoopGuard research-agent concept for ${profile.company_name}.`
      : 'A private TrustLoopGuard research-agent concept.',
    alternates: { canonical: profile?.demo_url ?? '/demo' },
    robots: { index: false, follow: false },
  };
}

export default async function PersonalizedResearchDemoPage({
  params,
}: PersonalizedResearchDemoPageProps) {
  const { company } = await params;
  const profile = await getResearchProfile(company);
  if (!profile) {
    notFound();
  }

  return <CompanyDemoPageContent profile={profile} />;
}

async function getResearchProfile(company: string) {
  const profile = await getActiveDemoProfile('research', company);
  return profile?.scenario_id === demoScenarioIdByCategory.research ? profile : null;
}
