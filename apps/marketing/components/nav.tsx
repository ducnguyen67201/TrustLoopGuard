import Link from 'next/link';
import { BOOK_MEETING_URL, GITHUB_URL, getStarCount } from '@/lib/github';
import { localizedHomeHref, type MarketingLocale } from '@/lib/marketing-locale';
import { MarketingEventLink } from './marketing-event-link';
import { NavActions } from './nav-actions';
import { UseCaseNav } from './use-case-nav';

const COPY = {
  en: {
    navigationLabel: 'Main navigation',
    homeLabel: 'Featherlane AI home',
    demo: 'Demo',
    demoEventLabel: 'Live demo',
    links: [
      { hash: '#how', label: 'How it works' },
    ],
  },
  vi: {
    navigationLabel: 'Điều hướng chính',
    homeLabel: 'Trang chủ Featherlane AI',
    demo: 'Dùng thử',
    demoEventLabel: 'Bản demo trực tiếp',
    links: [
      { hash: '#how', label: 'Cách hoạt động' },
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
          <img src="/featherlane-ai-logo.png" alt="" aria-hidden="true" className="wordmark-logo" />
          <span>Featherlane AI</span>
        </Link>
        <ul className="site-nav-links">
          <UseCaseNav locale={locale} />
          <li>
            <MarketingEventLink
              href={locale === 'vi' ? '/vi/demo' : '/demo'}
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
