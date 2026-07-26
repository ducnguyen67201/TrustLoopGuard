import { APP_URL } from '@/lib/app-url';
import type { MarketingLocale } from '@/lib/marketing-locale';
import { MarketingEventLink } from './marketing-event-link';
import { QuickInstall } from './quick-install';

const COPY = {
  en: {
    titleBefore: 'Policy',
    titleAccent: 'approvals',
    titleAfter: 'for AI agents.',
    intro:
      'TrustLoopGuard checks consequential actions, routes exceptions to the right person, and returns a decision before anything happens.',
    founderProof: 'Built by a former engineer at YC / a16z-backed companies',
    demo: 'Try the demo',
    app: 'Get started',
  },
  vi: {
    titleBefore: '',
    titleAccent: 'Phê duyệt',
    titleAfter: 'theo chính sách cho tác nhân AI.',
    intro:
      'TrustLoopGuard kiểm tra các hành động quan trọng, chuyển ngoại lệ đến đúng người và trả về quyết định trước khi bất kỳ điều gì xảy ra.',
    founderProof: 'Được xây dựng bởi cựu kỹ sư tại các công ty được YC / a16z hậu thuẫn',
    demo: 'Thử bản demo',
    app: 'Bắt đầu',
  },
} as const;

export function Hero({ locale = 'en' }: { locale?: MarketingLocale }) {
  const copy = COPY[locale];

  return (
    <section id="product" className="hero" aria-labelledby="hero-heading">
      <div className="hero-inner">
        <div className="hero-copy">
          <h1 id="hero-heading" className="hero-title">
            {copy.titleBefore !== '' ? `${copy.titleBefore} ` : null}
            <span>{copy.titleAccent}</span> {copy.titleAfter}
          </h1>
          <p className="hero-sub">{copy.intro}</p>
          <p className="hero-backing-proof">
            <span>{copy.founderProof}</span>
            <span className="hero-backing-logos">
              <span className="hero-backing-chip hero-backing-chip-yc">
                <img src="/yc-logo.svg" alt="Y Combinator" width="24" height="24" />
              </span>
              <span
                className="hero-backing-chip hero-backing-chip-a16z"
                role="img"
                aria-label="a16z"
              >
                <span aria-hidden="true">a16z</span>
              </span>
            </span>
          </p>
          <div className="hero-actions">
            <MarketingEventLink
              href="#demo"
              className="button-primary hero-action-button h-12"
              event="demo_click"
              eventParams={{
                page: locale === 'vi' ? '/vi' : '/',
                location: 'hero',
                label: copy.demo,
              }}
            >
              <PlayIcon />
              {copy.demo}
            </MarketingEventLink>
            <MarketingEventLink
              href={APP_URL}
              className="button-secondary hero-action-button h-12"
              event="app_click"
              eventParams={{
                page: locale === 'vi' ? '/vi' : '/',
                location: 'hero',
                label: copy.app,
              }}
            >
              {copy.app}
              <ArrowIcon />
            </MarketingEventLink>
          </div>
          <QuickInstall locale={locale} />
        </div>
      </div>
    </section>
  );
}

function PlayIcon() {
  return (
    <svg width="13" height="13" viewBox="0 0 13 13" fill="none" aria-hidden="true">
      <path d="M3 2.2L10.5 6.5 3 10.8V2.2Z" fill="currentColor" />
    </svg>
  );
}

function ArrowIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 14 14" fill="none" aria-hidden="true">
      <path d="M2.5 7H11.5M8 3.5L11.5 7L8 10.5" stroke="currentColor" strokeWidth="1.5" />
    </svg>
  );
}
