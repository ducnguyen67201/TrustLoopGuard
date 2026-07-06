'use client';

import { useState } from 'react';
import { trackMarketingEvent } from '@/lib/gtm';
import { Ascii } from './ascii-art';

type Status = 'idle' | 'sending' | 'ok' | 'error';

export function StayInTouch() {
  const [status, setStatus] = useState<Status>('idle');
  const [error, setError] = useState('');

  async function onSubmit(e: React.FormEvent<HTMLFormElement>) {
    e.preventDefault();
    const form = e.currentTarget;
    setStatus('sending');
    try {
      const res = await fetch('/api/subscribe', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify(Object.fromEntries(new FormData(form))),
      });
      const body = (await res.json().catch(() => ({}))) as { error?: string };
      if (!res.ok) throw new Error(body.error ?? 'Something broke — try again in a minute.');
      trackMarketingEvent('waitlist_submit', {
        page: '/',
        location: 'stay_in_touch',
        label: 'Notify me',
      });
      setStatus('ok');
      form.reset();
    } catch (err) {
      setStatus('error');
      setError(err instanceof Error ? err.message : 'Something broke — try again in a minute.');
    }
  }

  return (
    <section id="updates" aria-labelledby="stay-heading" className="section section-compact">
      <div className="card relative overflow-hidden p-8 md:p-10">
        <Ascii name="sparkA" className="ascii-drift absolute right-8 top-6 hidden md:block" />
        <p className="eyebrow">Stay in the loop</p>
        <h2 id="stay-heading" className="section-title">
          Get one email when something ships.
        </h2>
        <p className="section-copy mt-3 max-w-xl">
          New SDKs, policy features, launch dates. No spam, unsubscribe anytime.
        </p>

        {status === 'ok' ? (
          <p className="waitlist-ok mt-6" role="status">
            ✓ you&apos;re on the list
          </p>
        ) : (
          <form onSubmit={onSubmit} className="mt-6 flex max-w-xl flex-col gap-3 sm:flex-row">
            {/* honeypot — hidden from humans, catnip for bots */}
            <input
              type="text"
              name="company"
              tabIndex={-1}
              autoComplete="off"
              className="waitlist-trap"
              aria-hidden="true"
            />
            <input
              type="email"
              name="email"
              required
              placeholder="you@company.com"
              aria-label="Email address"
              className="waitlist-input h-12 flex-1 px-4"
            />
            <button
              type="submit"
              disabled={status === 'sending'}
              className="button-accent h-12 px-6"
            >
              {status === 'sending' ? 'Sending…' : 'Notify me'}
            </button>
          </form>
        )}
        {status === 'error' && (
          <p className="waitlist-err mt-3" role="alert">
            {error}
          </p>
        )}
      </div>
    </section>
  );
}
