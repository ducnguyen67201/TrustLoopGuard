import Link from 'next/link';

import type { CompanyDemoViewModel, OutboundDemoProfile } from './company-profile';
import { CONTEXTUAL_PAGE_COPY } from './contextual-content';
import { DemoAppLink } from './demo-app-link';
import styles from './demo.module.css';
import { CompanyDemo } from './[category]/company-demo';
import type { HealthcareDemoLocale } from './healthcare/content';

export function PersonalizedContextualDemoPageContent({
  profile,
  locale,
  pagePath,
}: {
  profile: OutboundDemoProfile;
  locale: HealthcareDemoLocale;
  pagePath: string;
}) {
  const copy = CONTEXTUAL_PAGE_COPY[locale];
  const demoProfile = toCompanyDemoViewModel(profile);

  return (
    <main className={styles['page']} lang={locale}>
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
          <span>{profile.company_name}</span>
          <i aria-hidden="true" />
          <span>{copy.personalizedConcept}</span>
        </div>
        <div className={styles['topbarActions']}>
          <Link
            href={locale === 'vi' ? '/vi/demo' : '/demo'}
            className={styles['topbarSecondaryLink']}
          >
            {copy.liveDemo} <span aria-hidden="true">↗</span>
          </Link>
          <DemoAppLink locale={locale} />
        </div>
      </header>

      <section className={styles['intro']} aria-labelledby="demo-title">
        <div>
          <p className={styles['eyebrow']}>{copy.preparedFor(profile.company_name)}</p>
          <h1 id="demo-title">{copy.heading}</h1>
        </div>
        <p className={styles['introCopy']}>{profile.risk_boundary}</p>
        <small className={styles['safetyNote']}>{copy.publicSource}</small>
      </section>

      <CompanyDemo
        profile={demoProfile}
        locale={locale}
        pagePath={pagePath}
      />
    </main>
  );
}

function toCompanyDemoViewModel(profile: OutboundDemoProfile): CompanyDemoViewModel {
  return {
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
}
