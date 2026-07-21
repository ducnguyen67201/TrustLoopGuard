import Link from 'next/link';
import type { CSSProperties } from 'react';

import type { OutboundDemoProfile } from '../company-profile';
import { DemoAppLink } from '../demo-app-link';
import styles from '../demo.module.css';
import { HEALTHCARE_PAGE_COPY, type HealthcareDemoLocale } from './content';
import { HealthcareDemo } from './healthcare-demo';

type PersonalizedHealthcareStyle = CSSProperties & {
  '--color-accent': string;
  '--color-accent-deep': string;
  '--color-accent-wash': string;
};

type HealthcareDemoPageContentProps = {
  locale: HealthcareDemoLocale;
  profile?: OutboundDemoProfile;
};

export function HealthcareDemoPageContent({
  locale,
  profile,
}: HealthcareDemoPageContentProps) {
  const copy = HEALTHCARE_PAGE_COPY[locale];
  const brandStyle: PersonalizedHealthcareStyle | undefined = profile
    ? {
        '--color-accent': profile.branding.primary_color,
        '--color-accent-deep': profile.branding.primary_color,
        '--color-accent-wash': profile.branding.secondary_color,
      }
    : undefined;

  return (
    <main className={styles['page']} lang={locale} style={brandStyle}>
      <header className={styles['topbar']}>
        <Link
          href={locale === 'vi' ? '/vi' : '/'}
          className={styles['wordmark']}
          aria-label={copy.homeLabel}
        >
          <img src="/trustloop-logo.svg" alt="" aria-hidden="true" />
          <span>TrustLoopGuard</span>
        </Link>
        <div className={styles['stackStatus']}>
          <span>{profile?.company_name ?? 'OpenAI Responses'}</span>
          <i aria-hidden="true" />
          <span>TrustLoopGuard</span>
          <i aria-hidden="true" />
          <span>{copy.deliveredReply}</span>
        </div>
        <div className={styles['topbarActions']}>
          <a
            href="https://github.com/ducnguyen67201/TrustLoopGuard"
            target="_blank"
            rel="noreferrer"
            className={styles['topbarSecondaryLink']}
          >
            {copy.viewSource} <span aria-hidden="true">↗</span>
          </a>
          <DemoAppLink locale={locale} />
        </div>
      </header>

      <section className={styles['intro']} aria-labelledby="healthcare-demo-title">
        <div>
          <p className={styles['eyebrow']}>
            {profile ? copy.preparedFor(profile.company_name) : copy.eyebrow}
          </p>
          <h1 id="healthcare-demo-title">
            {profile
              ? copy.personalizedHeading(profile.company_name)
              : copy.heading}
          </h1>
        </div>
        <p className={styles['introCopy']}>
          {profile?.risk_boundary ?? copy.introduction}
        </p>
        <small className={styles['safetyNote']} aria-label={copy.safetyLabel}>
          {copy.safetyNote}
        </small>
      </section>

      <HealthcareDemo
        locale={locale}
        presentation={
          profile
            ? { companyName: profile.company_name, workflow: profile.workflow }
            : undefined
        }
      />

      <footer className={styles['demoFooter']}>
        <p>
          {profile
            ? copy.personalizedDisclaimer(profile.company_name)
            : copy.disclaimer}
        </p>
        {!profile ? (
          <Link href={locale === 'vi' ? '/vi/demo' : '/demo'}>
            {copy.refundDemo} <span aria-hidden="true">→</span>
          </Link>
        ) : null}
      </footer>
    </main>
  );
}
