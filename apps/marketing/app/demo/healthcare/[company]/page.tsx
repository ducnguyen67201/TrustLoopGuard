import type { Metadata } from 'next';
import { notFound } from 'next/navigation';

import { getDemoProfile } from '../../../../lib/server/outbound-demo-profile-store';
import { demoScenarioIdByCategory } from '../../company-profile';
import { HealthcareDemoPageContent } from '../healthcare-page';

type PersonalizedHealthcareDemoPageProps = {
  params: Promise<{ company: string }>;
};

export const dynamic = 'force-dynamic';

export async function generateMetadata({
  params,
}: PersonalizedHealthcareDemoPageProps): Promise<Metadata> {
  const { company } = await params;
  const profile = await getHealthcareSchedulingProfile(company);

  return {
    title: profile
      ? `${profile.company_name} Healthcare Scheduling Concept`
      : 'Personalized Healthcare Scheduling Demo',
    description: profile
      ? `A private, public-source TrustLoopGuard healthcare scheduling concept for ${profile.company_name}.`
      : 'A private TrustLoopGuard healthcare scheduling concept.',
    alternates: { canonical: '/demo/healthcare' },
    robots: { index: false, follow: false },
  };
}

export default async function PersonalizedHealthcareDemoPage({
  params,
}: PersonalizedHealthcareDemoPageProps) {
  const { company } = await params;
  const profile = await getHealthcareSchedulingProfile(company);
  if (!profile) {
    notFound();
  }

  return <HealthcareDemoPageContent locale="en" profile={profile} />;
}

async function getHealthcareSchedulingProfile(company: string) {
  const profile = await getDemoProfile('healthcare', company);
  return profile?.scenario_id === demoScenarioIdByCategory.healthcare ? profile : null;
}
