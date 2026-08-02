import type { Metadata } from 'next';
import { notFound } from 'next/navigation';

import {
  demoScenarioIdByCategory,
  genericContextualScenarioId,
} from '@/app/demo/company-profile';
import { CONTEXTUAL_PAGE_COPY } from '@/app/demo/contextual-content';
import { HealthcareDemoPageContent } from '@/app/demo/healthcare/healthcare-page';
import { PersonalizedContextualDemoPageContent } from '@/app/demo/personalized-contextual-page';
import { getDemoProfile } from '@/lib/server/outbound-demo-profile-store';

type PersonalizedVietnameseHealthcareDemoPageProps = {
  params: Promise<{ company: string }>;
};

export const dynamic = 'force-dynamic';

export async function generateMetadata({
  params,
}: PersonalizedVietnameseHealthcareDemoPageProps): Promise<Metadata> {
  const { company } = await params;
  const profile = await getHealthcareProfile(company);
  const contextualCopy = CONTEXTUAL_PAGE_COPY.vi;
  const isContextual = profile?.scenario_id === genericContextualScenarioId;

  return {
    title: profile
      ? isContextual
        ? contextualCopy.title(profile.company_name)
        : `Bản thử nghiệm đặt lịch y tế an toàn cho ${profile.company_name}`
      : 'Bản thử nghiệm đặt lịch y tế được cá nhân hóa',
    description: profile
      ? isContextual
        ? contextualCopy.description(profile.company_name)
        : `Bản thử nghiệm Featherlane AI riêng cho quy trình đặt lịch của ${profile.company_name}, được xây dựng từ nguồn công khai.`
      : 'Bản thử nghiệm đặt lịch y tế an toàn của Featherlane AI.',
    alternates: {
      canonical: profile
        ? `/vi/demo/healthcare/${profile.slug}`
        : '/vi/demo/healthcare',
      languages: { 'vi-VN': profile ? `/vi/demo/healthcare/${profile.slug}` : '/vi/demo/healthcare' },
    },
    robots: { index: false, follow: false },
    openGraph: { locale: 'vi_VN' },
  };
}

export default async function PersonalizedVietnameseHealthcareDemoPage({
  params,
}: PersonalizedVietnameseHealthcareDemoPageProps) {
  const { company } = await params;
  const profile = await getHealthcareProfile(company);
  if (!profile) notFound();

  if (profile.scenario_id === genericContextualScenarioId) {
    return (
      <PersonalizedContextualDemoPageContent
        profile={profile}
        locale="vi"
        pagePath={`/vi/demo/healthcare/${profile.slug}`}
      />
    );
  }

  return <HealthcareDemoPageContent locale="vi" profile={profile} />;
}

async function getHealthcareProfile(company: string) {
  const profile = await getDemoProfile('healthcare', company);
  return profile?.scenario_id === demoScenarioIdByCategory.healthcare ||
    profile?.scenario_id === genericContextualScenarioId
    ? profile
    : null;
}
