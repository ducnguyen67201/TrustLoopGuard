import type { Metadata } from 'next';
import Link from 'next/link';

import styles from '../demo.module.css';
import { HealthcareDemo } from './healthcare-demo';

export const metadata: Metadata = {
  title: 'Secure Healthcare Scheduling Agent Demo',
  description:
    'Chat with a synthetic hospital scheduling agent and watch TrustLoopGuard check user input and OpenAI output before a reply is delivered.',
  alternates: { canonical: '/demo/healthcare' },
};

export default function HealthcareDemoPage() {
  return (
    <main className={styles['page']}>
      <header className={styles['topbar']}>
        <Link href="/" className={styles['wordmark']} aria-label="TrustLoopGuard home">
          <img src="/trustloop-logo.svg" alt="" aria-hidden="true" />
          <span>TrustLoopGuard</span>
        </Link>
        <div className={styles['stackStatus']}>
          <span>OpenAI Responses</span>
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
          <p className={styles['eyebrow']}>Protected scheduling agent</p>
          <h1 id="healthcare-demo-title">Chat with a protected hospital agent.</h1>
        </div>
        <p className={styles['introCopy']}>
          OpenAI drafts only after TrustLoopGuard permits the message, then the reply is checked
          again before delivery.
        </p>
        <small
          className={styles['safetyNote']}
          aria-label="Synthetic demo only — do not enter real patient information."
        >
          Synthetic demo only · No real PHI
        </small>
      </section>

      <HealthcareDemo />

      <footer className={styles['demoFooter']}>
        <p>
          This scheduling demo does not diagnose, access records, book appointments, or establish
          HIPAA compliance.
        </p>
        <Link href="/demo">
          Try the refund agent <span aria-hidden="true">→</span>
        </Link>
      </footer>
    </main>
  );
}
