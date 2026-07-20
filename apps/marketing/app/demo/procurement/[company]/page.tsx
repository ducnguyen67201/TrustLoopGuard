import type { Metadata } from 'next';
import { notFound } from 'next/navigation';

import { getDemoProfile } from '../../../../lib/server/outbound-demo-profile-store';
import { demoScenarioIdByCategory } from '../../company-profile';
import { ProcurementDemoPageContent } from '../procurement-page';

type PersonalizedProcurementDemoPageProps = {
  params: Promise<{ company: string }>;
};

export const dynamic = 'force-dynamic';

export async function generateMetadata({
  params,
}: PersonalizedProcurementDemoPageProps): Promise<Metadata> {
  const { company } = await params;
  const profile = await getProcurementProfile(company);

  return {
    title: profile
      ? `${profile.company_name} Procurement Concept`
      : 'Personalized Procurement Demo',
    description: profile
      ? `A private, public-source TrustLoopGuard procurement concept for ${profile.company_name}.`
      : 'A private TrustLoopGuard procurement concept.',
    alternates: { canonical: '/demo/procurement' },
    robots: { index: false, follow: false },
  };
}

export default async function PersonalizedProcurementDemoPage({
  params,
}: PersonalizedProcurementDemoPageProps) {
  const { company } = await params;
  const profile = await getProcurementProfile(company);
  if (!profile) {
    notFound();
  }

  return <ProcurementDemoPageContent locale="en" profile={profile} />;
}

async function getProcurementProfile(company: string) {
  const profile = await getDemoProfile('procurement', company);
  return profile?.scenario_id === demoScenarioIdByCategory.procurement ? profile : null;
}
