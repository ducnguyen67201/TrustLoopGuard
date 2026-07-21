import Link from 'next/link';
import type { CSSProperties } from 'react';

import type { MarketingLocale } from '@/lib/marketing-locale';

import type { OutboundDemoProfile } from '../company-profile';
import { DemoAppLink } from '../demo-app-link';
import sharedStyles from '../demo.module.css';
import { PROCUREMENT_DEMO_COPY } from './content';
import { ProcurementDemo } from './procurement-demo';
import styles from './procurement.module.css';

type PersonalizedProcurementStyle = CSSProperties & {
  '--color-accent': string;
  '--color-accent-deep': string;
  '--color-accent-wash': string;
};

type ProcurementDemoPageContentProps = {
  locale: MarketingLocale;
  profile?: OutboundDemoProfile;
};

export function ProcurementDemoPageContent({
  locale,
  profile,
}: ProcurementDemoPageContentProps) {
  const copy = PROCUREMENT_DEMO_COPY[locale];
  const brandStyle: PersonalizedProcurementStyle | undefined = profile
    ? {
        '--color-accent': profile.branding.primary_color,
        '--color-accent-deep': profile.branding.primary_color,
        '--color-accent-wash': `color-mix(in srgb, ${profile.branding.primary_color} 9%, transparent)`,
      }
    : undefined;

  return (
    <main className={sharedStyles['page']} lang={locale} style={brandStyle}>
      <header className={sharedStyles['topbar']}>
        <Link
          href={locale === 'vi' ? '/vi' : '/'}
          className={sharedStyles['wordmark']}
          aria-label="TrustLoopGuard home"
        >
          <img src="/trustloop-logo.svg" alt="" aria-hidden="true" />
          <span>TrustLoopGuard</span>
        </Link>
        <div className={sharedStyles['stackStatus']}>
          <span>{profile?.company_name ?? 'OpenAI Agents SDK'}</span>
          <i aria-hidden="true" />
          <span>TrustLoopGuard</span>
          <i aria-hidden="true" />
          <span>Demo procurement</span>
        </div>
        <div className={sharedStyles['topbarActions']}>
          <a
            href="https://github.com/ducnguyen67201/TrustLoopGuard"
            target="_blank"
            rel="noreferrer"
            className={sharedStyles['topbarSecondaryLink']}
          >
            View source <span aria-hidden="true">↗</span>
          </a>
          <DemoAppLink locale={locale} />
        </div>
      </header>

      <section
        className={`${sharedStyles['intro']} ${styles['procurementIntro']}`}
        aria-labelledby="procurement-demo-title"
      >
        <div>
          <p className={sharedStyles['eyebrow']}>
            {profile ? `Prepared for ${profile.company_name}` : 'Live procurement agent demo'}
          </p>
          <h1 id="procurement-demo-title" className={styles['title']}>
            {profile
              ? `${profile.company_name} procurement concept.`
              : 'OpenAI proposes. TrustLoopGuard decides before procurement executes.'}
          </h1>
        </div>
        <p className={sharedStyles['introCopy']}>
          {profile?.risk_boundary ??
            'Ask a live agent to source an item. It can search the demo catalog, but every purchase order must pass real TrustLoopGuard policy evaluation before the procurement system is called.'}
        </p>
        <small className={sharedStyles['safetyNote']} aria-label={copy.safetyLabel}>
          {copy.safetyNote}
        </small>
      </section>

      <ProcurementDemo
        locale={locale}
        presentation={
          profile
            ? { companyName: profile.company_name, workflow: profile.workflow }
            : undefined
        }
      />

      <footer className={sharedStyles['demoFooter']}>
        <p>
          {profile?.disclaimer ?? 'Give AI procurement tools a deterministic control boundary.'}
        </p>
        {!profile ? (
          <Link href="/">
            Explore TrustLoopGuard <span aria-hidden="true">→</span>
          </Link>
        ) : null}
      </footer>
    </main>
  );
}
