import type { Metadata } from 'next';
import { notFound } from 'next/navigation';

import { getDemoProfile } from '../../../../lib/server/outbound-demo-profile-store';
import {
  demoScenarioIdByCategory,
  genericContextualScenarioId,
} from '../../company-profile';
import { PersonalizedContextualDemoPageContent } from '../../personalized-contextual-page';
import { HealthcareDemoPageContent } from '../healthcare-page';

type PersonalizedHealthcareDemoPageProps = {
  params: Promise<{ company: string }>;
};

export const dynamic = 'force-dynamic';

export async function generateMetadata({
  params,
}: PersonalizedHealthcareDemoPageProps): Promise<Metadata> {
  const { company } = await params;
  const profile = await getHealthcareProfile(company);
  const isContextual = profile?.scenario_id === genericContextualScenarioId;

  return {
    title: profile
      ? isContextual
        ? `${profile.company_name} AI Guardrail Concept`
        : `${profile.company_name} Healthcare Scheduling Concept`
      : 'Personalized Healthcare Scheduling Demo',
    description: profile
      ? isContextual
        ? `A private, public-source TrustLoopGuard concept for ${profile.company_name}.`
        : `A private, public-source TrustLoopGuard healthcare scheduling concept for ${profile.company_name}.`
      : 'A private TrustLoopGuard healthcare scheduling concept.',
    alternates: { canonical: '/demo/healthcare' },
    robots: { index: false, follow: false },
  };
}

export default async function PersonalizedHealthcareDemoPage({
  params,
}: PersonalizedHealthcareDemoPageProps) {
  const { company } = await params;
  const profile = await getHealthcareProfile(company);
  if (!profile) {
    notFound();
  }

  if (profile.scenario_id === genericContextualScenarioId) {
    return (
      <PersonalizedContextualDemoPageContent
        profile={profile}
        locale="en"
        pagePath={`/demo/healthcare/${profile.slug}`}
      />
    );
  }

  return <HealthcareDemoPageContent locale="en" profile={profile} />;
}

async function getHealthcareProfile(company: string) {
  const profile = await getDemoProfile('healthcare', company);
  return profile?.scenario_id === demoScenarioIdByCategory.healthcare ||
    profile?.scenario_id === genericContextualScenarioId
    ? profile
    : null;
}
