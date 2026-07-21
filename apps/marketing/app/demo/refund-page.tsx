import Link from 'next/link';

import type { MarketingLocale } from '@/lib/marketing-locale';

import styles from './demo.module.css';
import { DemoAppLink } from './demo-app-link';
import { REFUND_PAGE_COPY } from './refund-content';
import { RefundDemo } from './refund-demo';

export function RefundDemoPageContent({ locale }: { locale: MarketingLocale }) {
  const copy = REFUND_PAGE_COPY[locale];

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
          <span>OpenAI</span>
          <i aria-hidden="true" />
          <span>Rust API</span>
          <i aria-hidden="true" />
          <span>{copy.stripeTestMode}</span>
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

      <section className={styles['intro']} aria-labelledby="demo-title">
        <div>
          <p className={styles['eyebrow']}>{copy.eyebrow}</p>
          <h1 id="demo-title">{copy.heading}</h1>
        </div>
        <p className={styles['introCopy']}>
          {copy.introduction}{' '}
          <Link href={locale === 'vi' ? '/vi/demo/healthcare' : '/demo/healthcare'}>
            {copy.healthcareDemo} →
          </Link>
        </p>
        <small className={styles['safetyNote']} aria-label={copy.safetyLabel}>
          {copy.safetyNote}
        </small>
      </section>

      <RefundDemo locale={locale} />
    </main>
  );
}
