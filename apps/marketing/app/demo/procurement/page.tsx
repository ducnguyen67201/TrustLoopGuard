import type { Metadata } from 'next';
import Link from 'next/link';

import sharedStyles from '../demo.module.css';
import { ProcurementDemo } from './procurement-demo';
import styles from './procurement.module.css';

export const metadata: Metadata = {
  title: 'Secure AI Procurement Agent Demo',
  description:
    'Chat with a live OpenAI procurement agent and watch TrustLoopGuard permit, hold, or block purchase orders before they execute.',
  alternates: {
    canonical: '/demo/procurement',
    languages: {
      en: '/demo/procurement',
      vi: '/vi/demo/procurement',
    },
  },
};

export default function ProcurementDemoPage() {
  return (
    <main className={sharedStyles['page']}>
      <header className={sharedStyles['topbar']}>
        <Link href="/" className={sharedStyles['wordmark']} aria-label="TrustLoopGuard home">
          <img src="/trustloop-logo.svg" alt="" aria-hidden="true" />
          <span>TrustLoopGuard</span>
        </Link>
        <div className={sharedStyles['stackStatus']}>
          <span>OpenAI Agents SDK</span>
          <i aria-hidden="true" />
          <span>Rust API</span>
          <i aria-hidden="true" />
          <span>Demo procurement</span>
        </div>
        <a href="https://github.com/ducnguyen67201/TrustLoopGuard" target="_blank" rel="noreferrer">
          View source <span aria-hidden="true">↗</span>
        </a>
      </header>

      <section
        className={`${sharedStyles['intro']} ${styles['procurementIntro']}`}
        aria-labelledby="procurement-demo-title"
      >
        <div>
          <p className={sharedStyles['eyebrow']}>Live procurement agent demo</p>
          <h1 id="procurement-demo-title" className={styles['title']}>
            OpenAI proposes. TrustLoopGuard decides before procurement executes.
          </h1>
        </div>
        <div className={sharedStyles['introCopy']}>
          <p>
            Ask a live agent to source an item. It can search the demo catalog, but every purchase
            order must pass real TrustLoopGuard policy evaluation before the procurement system is
            called.
          </p>
          <small>Built with OpenAI’s Agents SDK and protected by TrustLoopGuard.</small>
        </div>
      </section>

      <div className={styles['dataNotice']} role="note">
        Demo catalog only. Do not enter confidential procurement, supplier, or commercial
        information.
      </div>

      <ProcurementDemo locale="en" />

      <footer className={sharedStyles['demoFooter']}>
        <p>Give AI procurement tools a deterministic control boundary.</p>
        <Link href="/">
          Explore TrustLoopGuard <span aria-hidden="true">→</span>
        </Link>
      </footer>
    </main>
  );
}
