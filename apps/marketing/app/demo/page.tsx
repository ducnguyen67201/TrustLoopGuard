import type { Metadata } from 'next';
import Link from 'next/link';
import { RefundDemo } from './refund-demo';
import styles from './demo.module.css';

export const metadata: Metadata = {
  title: 'Live AI Refund Guard Demo',
  description:
    'Ask a live AI support agent to issue a refund and watch TrustLoopGuard authorize, hold, or block the Stripe action before it executes.',
  alternates: { canonical: '/demo' },
};

export default function DemoPage() {
  return (
    <main className={styles['page']}>
      <header className={styles['topbar']}>
        <Link href="/" className={styles['wordmark']} aria-label="TrustLoopGuard home">
          <img src="/trustloop-logo.svg" alt="" aria-hidden="true" />
          <span>TrustLoopGuard</span>
        </Link>
        <div className={styles['stackStatus']}>
          <span>OpenAI</span>
          <i aria-hidden="true" />
          <span>Rust API</span>
          <i aria-hidden="true" />
          <span>Stripe test mode</span>
        </div>
        <a href="https://github.com/ducnguyen67201/TrustLoopGuard" target="_blank" rel="noreferrer">
          View source <span aria-hidden="true">↗</span>
        </a>
      </header>

      <section className={styles['intro']} aria-labelledby="demo-title">
        <div>
          <p className={styles['eyebrow']}>Live refund demo</p>
          <h1 id="demo-title">Ask the agent. Watch the guard decide.</h1>
        </div>
        <p className={styles['introCopy']}>
          Pick an amount below. TrustLoopGuard allows, holds, or blocks the agent before Stripe —
          real APIs, not a scripted animation.
        </p>
        <small className={styles['safetyNote']}>Test data · No real money</small>
      </section>

      <RefundDemo />
    </main>
  );
}
