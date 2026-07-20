import type { Metadata } from 'next';
import Link from 'next/link';
import type { CSSProperties } from 'react';

import type { OutboundDemoProfile } from '../company-profile';
import styles from '../demo.module.css';
import { HealthcareDemo } from './healthcare-demo';

export const metadata: Metadata = {
  title: 'Secure Healthcare Scheduling Agent Demo',
  description:
    'Chat with a synthetic hospital scheduling agent and watch TrustLoopGuard check user input and OpenAI output before a reply is delivered.',
  alternates: { canonical: '/demo/healthcare' },
};

export default function HealthcareDemoPage() {
  return <HealthcareDemoPageView />;
}

type PersonalizedHealthcareStyle = CSSProperties & {
  '--color-accent': string;
  '--color-accent-deep': string;
  '--color-accent-wash': string;
};

export function HealthcareDemoPageView({ profile }: { profile?: OutboundDemoProfile }) {
  const brandStyle: PersonalizedHealthcareStyle | undefined = profile
    ? {
        '--color-accent': profile.branding.primary_color,
        '--color-accent-deep': profile.branding.primary_color,
        '--color-accent-wash': profile.branding.secondary_color,
      }
    : undefined;

  return (
    <main className={styles['page']} style={brandStyle}>
      <header className={styles['topbar']}>
        <Link href="/" className={styles['wordmark']} aria-label="TrustLoopGuard home">
          <img src="/trustloop-logo.svg" alt="" aria-hidden="true" />
          <span>TrustLoopGuard</span>
        </Link>
        <div className={styles['stackStatus']}>
          <span>{profile?.company_name ?? 'OpenAI Responses'}</span>
          <i aria-hidden="true" />
          <span>TrustLoopGuard</span>
          <i aria-hidden="true" />
          <span>Delivered reply</span>
        </div>
        <a
          href="https://github.com/ducnguyen67201/TrustLoopGuard"
          target="_blank"
          rel="noreferrer"
        >
          View source <span aria-hidden="true">↗</span>
        </a>
      </header>

      <section className={styles['intro']} aria-labelledby="healthcare-demo-title">
        <div>
          <p className={styles['eyebrow']}>
            {profile ? `Prepared for ${profile.company_name}` : 'Protected scheduling agent'}
          </p>
          <h1 id="healthcare-demo-title">
            {profile
              ? `${profile.company_name} healthcare scheduling concept.`
              : 'Chat with a protected hospital agent.'}
          </h1>
        </div>
        <p className={styles['introCopy']}>
          {profile?.risk_boundary ??
            'OpenAI drafts only after TrustLoopGuard permits the message, then the reply is checked again before delivery.'}
        </p>
        <small
          className={styles['safetyNote']}
          aria-label="Synthetic demo only — do not enter real patient information."
        >
          Synthetic demo only · No real PHI
        </small>
      </section>

      <HealthcareDemo
        presentation={
          profile
            ? { companyName: profile.company_name, workflow: profile.workflow }
            : undefined
        }
      />

      <footer className={styles['demoFooter']}>
        <p>
          {profile?.disclaimer ??
            'This scheduling demo does not diagnose, access records, book appointments, or establish HIPAA compliance.'}
        </p>
        {profile ? (
          <div className={styles['demoFooterSources']}>
            {profile.sources.map((source) => (
              <a key={source.url} href={source.url} target="_blank" rel="noreferrer">
                {source.title} <span aria-hidden="true">↗</span>
              </a>
            ))}
          </div>
        ) : (
          <Link href="/demo">
            Try the refund agent <span aria-hidden="true">→</span>
          </Link>
        )}
      </footer>
    </main>
  );
}
