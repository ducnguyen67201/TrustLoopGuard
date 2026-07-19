import Link from 'next/link';

import styles from '../demo.module.css';
import { HEALTHCARE_PAGE_COPY, type HealthcareDemoLocale } from './content';
import { HealthcareDemo } from './healthcare-demo';

export function HealthcareDemoPageContent({ locale }: { locale: HealthcareDemoLocale }) {
  const copy = HEALTHCARE_PAGE_COPY[locale];

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
          <span>OpenAI Responses</span>
          <i aria-hidden="true" />
          <span>TrustLoopGuard</span>
          <i aria-hidden="true" />
          <span>{copy.deliveredReply}</span>
        </div>
        <a
          href="https://github.com/ducnguyen67201/TrustLoopGuard"
          target="_blank"
          rel="noreferrer"
        >
          {copy.viewSource} <span aria-hidden="true">↗</span>
        </a>
      </header>

      <section className={styles['intro']} aria-labelledby="healthcare-demo-title">
        <div>
          <p className={styles['eyebrow']}>{copy.eyebrow}</p>
          <h1 id="healthcare-demo-title">{copy.heading}</h1>
        </div>
        <p className={styles['introCopy']}>{copy.introduction}</p>
        <small className={styles['safetyNote']} aria-label={copy.safetyLabel}>
          {copy.safetyNote}
        </small>
      </section>

      <HealthcareDemo locale={locale} />

      <footer className={styles['demoFooter']}>
        <p>{copy.disclaimer}</p>
        <Link href="/demo">
          {copy.refundDemo} <span aria-hidden="true">→</span>
        </Link>
      </footer>
    </main>
  );
}
