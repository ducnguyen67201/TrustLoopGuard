'use client';

import { useState } from 'react';
import { usePathname } from 'next/navigation';
import { BOOK_MEETING_URL, DOCS_URL, GITHUB_URL } from '@/lib/github';
import { trackMarketingEvent } from '@/lib/gtm';
import type { MarketingLocale } from '@/lib/marketing-locale';
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
      { href: '/use-cases', label: 'All use cases' },
      { href: '/use-cases/shell-command-safety', label: 'Shell command safety' },
      { href: '/use-cases/email', label: 'Outbound email' },
      { href: '/use-cases/agent-spending-caps', label: 'Agent spending caps' },
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

const LINK_GROUPS_VI = [
  {
    title: 'Sản phẩm',
    links: [
      { href: '/ai-agent-spend-controls', label: 'Kiểm soát chi tiêu của tác nhân AI' },
      { href: '/ai-agent-payment-gateway', label: 'Hàng rào cho cổng thanh toán' },
      { href: '/mcp-spend-guard', label: 'Bảo vệ chi tiêu MCP' },
      { href: '/ai-agent-audit-trail', label: 'Nhật ký kiểm toán tác nhân' },
    ],
  },
  {
    title: 'Tình huống sử dụng',
    links: [
      { href: '/use-cases', label: 'Tất cả tình huống' },
      { href: '/use-cases/shell-command-safety', label: 'An toàn lệnh shell' },
      { href: '/use-cases/email', label: 'Email gửi đi' },
      { href: '/use-cases/agent-spending-caps', label: 'Hạn mức chi tiêu tác nhân' },
    ],
  },
  {
    title: 'Tài nguyên',
    links: [
      { href: '/vi#developers', label: 'Bắt đầu nhanh cho nhà phát triển' },
      { href: '/vi#product', label: 'Hợp đồng quyết định' },
      { href: DOCS_URL, label: 'Tài liệu' },
      { href: GITHUB_URL, label: 'GitHub' },
    ],
  },
  {
    title: 'Công ty',
    links: [
      { href: '/vi#trust', label: 'Vì sao chọn TrustLoopGuard' },
      { href: BOOK_MEETING_URL, label: 'Đặt lịch demo' },
      { href: '/vi#updates', label: 'Cập nhật sản phẩm' },
    ],
  },
] as const;

const COPY = {
  en: {
    tagline: 'Runtime control for production AI agents.',
    productHuntLabel: 'View TrustLoopGuard on Product Hunt',
    productHuntAlt: 'TrustLoopGuard - Control AI agents before irreversible actions | Product Hunt',
    footerNavigation: 'Footer navigation',
    newsletterHeading: 'Occasional product notes',
    newsletterCopy: 'New SDKs, policy features, and practical notes from the failure path.',
    subscribed: 'You are on the list.',
    emailPlaceholder: 'Your email',
    emailLabel: 'Email address',
    sending: 'Sending',
    subscribe: 'Subscribe',
    error: 'Could not subscribe. Try again in a minute.',
    linksLabel: 'TrustLoopGuard links',
    openSource: 'Apache-2.0 open source',
    builtInOpen: 'Built in the open',
  },
  vi: {
    tagline: 'Kiểm soát tác nhân AI trong môi trường production.',
    productHuntLabel: 'Xem TrustLoopGuard trên Product Hunt',
    productHuntAlt:
      'TrustLoopGuard - Kiểm soát tác nhân AI trước các hành động không thể đảo ngược | Product Hunt',
    footerNavigation: 'Điều hướng cuối trang',
    newsletterHeading: 'Cập nhật sản phẩm định kỳ',
    newsletterCopy: 'SDK mới, tính năng chính sách và ghi chú thực tế từ các đường dẫn lỗi.',
    subscribed: 'Bạn đã có trong danh sách.',
    emailPlaceholder: 'Email của bạn',
    emailLabel: 'Địa chỉ email',
    sending: 'Đang gửi',
    subscribe: 'Đăng ký',
    error: 'Không thể đăng ký. Vui lòng thử lại sau ít phút.',
    linksLabel: 'Các liên kết TrustLoopGuard',
    openSource: 'Mã nguồn mở Apache-2.0',
    builtInOpen: 'Được xây dựng công khai',
  },
} as const;

type Status = 'idle' | 'sending' | 'ok' | 'error';

export function Footer({ locale = 'en' }: { locale?: MarketingLocale }) {
  const [status, setStatus] = useState<Status>('idle');
  const [error, setError] = useState('');
  const page = usePathname() || '/';
  const copy = COPY[locale];
  const linkGroups = locale === 'vi' ? LINK_GROUPS_VI : LINK_GROUPS;

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
      if (!res.ok) throw new Error(body.error ?? copy.error);

      trackMarketingEvent('waitlist_submit', {
        page: window.location.pathname,
        location: 'footer',
        label: 'Subscribe',
      });
      setStatus('ok');
      form.reset();
    } catch (err) {
      setStatus('error');
      setError(err instanceof Error ? err.message : copy.error);
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
          <div className="footer-intro-aside">
            <p>{copy.tagline}</p>
            <a
              href="https://www.producthunt.com/products/trustloopguard?embed=true&utm_source=badge-featured&utm_medium=badge&utm_campaign=badge-trustloopguard"
              target="_blank"
              rel="noopener noreferrer"
              className="product-hunt-badge"
              aria-label={copy.productHuntLabel}
            >
              <img
                src="https://api.producthunt.com/widgets/embed-image/v1/featured.svg?post_id=1195728&theme=light&t=1783984627137"
                alt={copy.productHuntAlt}
                width="250"
                height="54"
              />
            </a>
          </div>
        </div>
        <div className="grid gap-9 lg:grid-cols-[1fr_22rem]">
          <nav
            aria-label={copy.footerNavigation}
            className="grid gap-8 sm:grid-cols-2 xl:grid-cols-4"
          >
            {linkGroups.map((group) => (
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

          <section
            id="updates"
            className="footer-link-group"
            aria-labelledby="footer-newsletter-heading"
          >
            <div className="footer-rule" />
            <h2 id="footer-newsletter-heading">{copy.newsletterHeading}</h2>
            <p className="footer-newsletter-copy">{copy.newsletterCopy}</p>
            {status === 'ok' ? (
              <p className="footer-form-status footer-form-ok" role="status">
                {copy.subscribed}
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
                  placeholder={copy.emailPlaceholder}
                  aria-label={copy.emailLabel}
                  className="footer-email-input"
                />
                <button type="submit" disabled={status === 'sending'} className="footer-submit">
                  {status === 'sending' ? copy.sending : copy.subscribe}
                </button>
              </form>
            )}
            {status === 'error' && (
              <p className="footer-form-status footer-form-error" role="alert">
                {error}
              </p>
            )}
            <div className="footer-socials" aria-label={copy.linksLabel}>
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
            {copy.openSource}
          </p>
          <div>
            <span>{copy.builtInOpen}</span>
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
