import type { Metadata } from 'next';
import Link from 'next/link';
import { notFound } from 'next/navigation';

import { getDemoProfile } from '../../../../lib/server/outbound-demo-profile-store';
import { CompanyDemo } from '../../[category]/company-demo';
import {
  demoScenarioIdByCategory,
  genericContextualScenarioId,
  type CompanyDemoViewModel,
  type OutboundDemoProfile,
} from '../../company-profile';
import styles from '../../demo.module.css';
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
    return <HealthcareContextualDemoPage profile={profile} />;
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

function HealthcareContextualDemoPage({ profile }: { profile: OutboundDemoProfile }) {
  const demoProfile: CompanyDemoViewModel = {
    slug: profile.slug,
    company_name: profile.company_name,
    scenario_id: profile.scenario_id,
    user_profile: profile.user_profile,
    workflow: profile.workflow,
    risk_boundary: profile.risk_boundary,
    rule: profile.rule,
    approval_step: profile.approval_step,
    record_shown: profile.record_shown,
    branding: {
      primary_color: profile.branding.primary_color,
      secondary_color: profile.branding.secondary_color,
    },
    paths: profile.paths,
    disclaimer: profile.disclaimer,
  };

  return (
    <main className={styles['page']}>
      <header className={styles['topbar']}>
        <Link href="/" className={styles['wordmark']} aria-label="TrustLoopGuard home">
          <img src="/trustloop-logo.svg" alt="" aria-hidden="true" />
          <span>TrustLoopGuard</span>
        </Link>
        <div className={styles['stackStatus']}>
          <span>{profile.company_name}</span>
          <i aria-hidden="true" />
          <span>Personalized concept</span>
        </div>
        <Link href="/demo">
          Live product demo <span aria-hidden="true">↗</span>
        </Link>
      </header>

      <section className={styles['intro']} aria-labelledby="demo-title">
        <div>
          <p className={styles['eyebrow']}>Prepared for {profile.company_name}</p>
          <h1 id="demo-title">Your workflow. A policy boundary before the action.</h1>
        </div>
        <p className={styles['introCopy']}>{profile.risk_boundary}</p>
        <small className={styles['safetyNote']}>Public-source concept</small>
      </section>

      <CompanyDemo profile={demoProfile} />
    </main>
  );
}
