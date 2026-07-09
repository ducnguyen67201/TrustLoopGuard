'use client';

import { useState } from 'react';
import { usePathname } from 'next/navigation';
import { BOOK_MEETING_URL, DOCS_URL, GITHUB_URL } from '@/lib/github';
import { trackMarketingEvent } from '@/lib/gtm';
import { MarketingEventLink } from './marketing-event-link';

const LINK_GROUPS = [
  {
    title: 'Products',
    links: [
      { href: '/ai-agent-spend-controls', label: 'AI agent spend controls' },
      { href: '/ai-agent-payment-gateway', label: 'Payment gateway guard' },
      { href: '/mcp-spend-guard', label: 'MCP spend guard' },
      { href: '/ai-agent-audit-trail', label: 'Agent audit trail' },
    ],
  },
  {
    title: 'Use cases',
    links: [
      { href: '/agentic-travel-payments', label: 'Travel payments' },
      { href: '/shopping-agent-checkout', label: 'Shopping checkout' },
      { href: '/accounts-payable-agents', label: 'Accounts payable agents' },
    ],
  },
  {
    title: 'Resources',
    links: [
      { href: '/#developers', label: 'Developer quickstart' },
      { href: '/#product', label: 'Decision contract' },
      { href: DOCS_URL, label: 'Docs' },
      { href: GITHUB_URL, label: 'GitHub' },
    ],
  },
  {
    title: 'Company',
    links: [
      { href: '/#trust', label: 'Why TrustLoopGuard' },
      { href: BOOK_MEETING_URL, label: 'Book a demo' },
      { href: '/#updates', label: 'Product notes' },
    ],
  },
] as const;

type Status = 'idle' | 'sending' | 'ok' | 'error';

export function Footer() {
  const [status, setStatus] = useState<Status>('idle');
  const [error, setError] = useState('');
  const page = usePathname() || '/';

  async function onSubmit(e: React.FormEvent<HTMLFormElement>) {
    e.preventDefault();
    const form = e.currentTarget;
    setStatus('sending');
    setError('');

    try {
      const res = await fetch('/api/subscribe', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify(Object.fromEntries(new FormData(form))),
      });
      const body = (await res.json().catch(() => ({}))) as { error?: string };
      if (!res.ok) throw new Error(body.error ?? 'Could not subscribe. Try again in a minute.');

      trackMarketingEvent('waitlist_submit', {
        page: window.location.pathname,
        location: 'footer',
        label: 'Subscribe',
      });
      setStatus('ok');
      form.reset();
    } catch (err) {
      setStatus('error');
      setError(err instanceof Error ? err.message : 'Could not subscribe. Try again in a minute.');
    }
  }

  return (
    <footer className="site-footer">
      <div className="footer-panel">
        <div className="footer-intro">
          <div className="wordmark footer-wordmark">
            <img src="/trustloop-logo.svg" alt="" aria-hidden="true" className="wordmark-logo" />
            <span>TrustLoopGuard</span>
          </div>
          <p>Runtime control for production AI agents.</p>
        </div>
        <div className="grid gap-9 lg:grid-cols-[1fr_22rem]">
          <nav
            aria-label="Footer navigation"
            className="grid gap-8 sm:grid-cols-2 xl:grid-cols-4"
          >
            {LINK_GROUPS.map((group) => (
              <section key={group.title} className="footer-link-group">
                <div className="footer-rule" />
                <h2>{group.title}</h2>
                <ul>
                  {group.links.map((link) => (
                    <li key={link.href}>
                      <MarketingEventLink
                        href={link.href}
                        target={link.href.startsWith('http') ? '_blank' : undefined}
                        event={getFooterEvent(link.href)}
                        eventParams={{ page, location: 'footer', label: link.label }}
                      >
                        {link.label}
                      </MarketingEventLink>
                    </li>
                  ))}
                </ul>
              </section>
            ))}
          </nav>

          <section id="updates" className="footer-link-group" aria-labelledby="footer-newsletter-heading">
            <div className="footer-rule" />
            <h2 id="footer-newsletter-heading">Occasional product notes</h2>
            <p className="footer-newsletter-copy">
              New SDKs, policy features, and practical notes from the failure path.
            </p>
            {status === 'ok' ? (
              <p className="footer-form-status footer-form-ok" role="status">
                You are on the list.
              </p>
            ) : (
              <form onSubmit={onSubmit} className="footer-form">
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
                  placeholder="Your email"
                  aria-label="Email address"
                  className="footer-email-input"
                />
                <button type="submit" disabled={status === 'sending'} className="footer-submit">
                  {status === 'sending' ? 'Sending' : 'Subscribe'}
                </button>
              </form>
            )}
            {status === 'error' && (
              <p className="footer-form-status footer-form-error" role="alert">
                {error}
              </p>
            )}
            <div className="footer-socials" aria-label="TrustLoopGuard links">
              <MarketingEventLink
                href={GITHUB_URL}
                target="_blank"
                event="github_click"
                eventParams={{ page, location: 'footer_socials', label: 'GH' }}
              >
                GH
              </MarketingEventLink>
              <MarketingEventLink
                href={DOCS_URL}
                target="_blank"
                event="docs_click"
                eventParams={{ page, location: 'footer_socials', label: 'Docs' }}
              >
                Docs
              </MarketingEventLink>
            </div>
          </section>
        </div>

        <div className="footer-bottom">
          <p>
            <span className="footer-status-dot" aria-hidden="true" />
            Apache-2.0 open source
          </p>
          <div>
            <span>Built in the open</span>
            <span>© 2026 TrustLoopGuard</span>
          </div>
        </div>
      </div>
    </footer>
  );
}

function getFooterEvent(href: string) {
  if (href === GITHUB_URL) return 'github_click';
  if (href === DOCS_URL) return 'docs_click';
  if (href === BOOK_MEETING_URL) return 'book_meeting_click';
  return 'landing_cta_click';
}
