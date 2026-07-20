import Link from 'next/link';
import { BOOK_MEETING_URL, GITHUB_URL, getStarCount } from '@/lib/github';
import { localizedHomeHref, type MarketingLocale } from '@/lib/marketing-locale';
import { MarketingEventLink } from './marketing-event-link';
import { NavActions } from './nav-actions';
import { UseCaseNav } from './use-case-nav';

const COPY = {
  en: {
    navigationLabel: 'Main navigation',
    homeLabel: 'TrustLoopGuard home',
    product: 'Product',
    demo: 'Demo',
    demoEventLabel: 'Live demo',
    links: [
      { hash: '#trust', label: 'Why trust us' },
      { hash: '#how', label: 'How it works' },
      { hash: '#developers', label: 'Developers' },
    ],
  },
  vi: {
    navigationLabel: 'Điều hướng chính',
    homeLabel: 'Trang chủ TrustLoopGuard',
    product: 'Sản phẩm',
    demo: 'Dùng thử',
    demoEventLabel: 'Bản demo trực tiếp',
    links: [
      { hash: '#trust', label: 'Vì sao tin tưởng' },
      { hash: '#how', label: 'Cách hoạt động' },
      { hash: '#developers', label: 'Nhà phát triển' },
    ],
  },
} as const;

export async function Nav({ locale = 'en' }: { locale?: MarketingLocale }) {
  const stars = await getStarCount();
  const copy = COPY[locale];
  const homeHref = localizedHomeHref(locale);

  return (
    <header className="site-header sticky top-0 inset-x-0 z-40">
      <nav aria-label={copy.navigationLabel} className="site-nav">
        <Link href={homeHref} className="wordmark" aria-label={copy.homeLabel}>
          <img src="/trustloop-logo.svg" alt="" aria-hidden="true" className="wordmark-logo" />
          <span>TrustLoopGuard</span>
        </Link>
        <ul className="site-nav-links">
          <li>
            <a href={localizedHomeHref(locale, '#product')}>{copy.product}</a>
          </li>
          <UseCaseNav locale={locale} />
          <li>
            <MarketingEventLink
              href={locale === 'vi' ? '/vi/demo/procurement' : '/demo'}
              event="demo_click"
              eventParams={{ location: 'nav', label: copy.demoEventLabel }}
            >
              {copy.demo}
            </MarketingEventLink>
          </li>
          {copy.links.map((link) => (
            <li key={link.hash}>
              <a href={localizedHomeHref(locale, link.hash)}>{link.label}</a>
            </li>
          ))}
        </ul>
        <NavActions
          bookMeetingUrl={BOOK_MEETING_URL}
          githubUrl={GITHUB_URL}
          stars={stars}
          locale={locale}
        />
      </nav>
    </header>
  );
}
