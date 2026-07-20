import type { Metadata } from 'next';
import Link from 'next/link';
import { notFound } from 'next/navigation';

import { getDemoProfileBySlug } from '../../../lib/server/outbound-demo-profile-store';
import type { CompanyDemoViewModel } from '../company-profile';
import styles from '../demo.module.css';
import { CompanyDemo } from './company-demo';

type CompanyDemoPageProps = {
  params: Promise<{ company: string }>;
};

export const dynamic = 'force-dynamic';

export async function generateMetadata({ params }: CompanyDemoPageProps): Promise<Metadata> {
  const { company } = await params;
  const profile = await getDemoProfileBySlug(company);

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
  const profile = await getDemoProfileBySlug(company);
  if (!profile) {
    notFound();
  }

  const demoProfile: CompanyDemoViewModel = {
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
    sources: profile.sources,
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
