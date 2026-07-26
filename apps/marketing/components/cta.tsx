import { BOOK_MEETING_URL, DOCS_URL, GITHUB_URL } from '@/lib/github';
import type { MarketingLocale } from '@/lib/marketing-locale';
import { MarketingEventLink } from './marketing-event-link';

const COPY = {
  en: {
    eyebrow: 'Start with one approval',
    title: 'What should your agent never do alone?',
    body: 'We will map the policy and approval path.',
    talk: 'Talk to founder',
    source: 'Review the source',
    docs: 'Read the docs ↗',
  },
  vi: {
    eyebrow: 'Bắt đầu với một phê duyệt',
    title: 'Tác nhân của bạn không bao giờ nên tự làm gì?',
    body: 'Chúng tôi sẽ lập bản đồ chính sách và luồng phê duyệt.',
    talk: 'Trao đổi với nhà sáng lập',
    source: 'Xem mã nguồn',
    docs: 'Đọc tài liệu ↗',
  },
} as const;

export function Cta({ locale = 'en' }: { locale?: MarketingLocale }) {
  const copy = COPY[locale];
  const page = locale === 'vi' ? '/vi' : '/';

  return (
    <section aria-labelledby="cta-heading" className="section cta-section">
      <div className="cta-card">
        <div>
          <p className="eyebrow eyebrow-light">{copy.eyebrow}</p>
          <h2 id="cta-heading">{copy.title}</h2>
        </div>
        <div>
          <p>{copy.body}</p>
          <div className="cta-actions">
            <MarketingEventLink
              href={BOOK_MEETING_URL}
              target="_blank"
              className="button-invert h-12 px-6"
              event="book_meeting_click"
              eventParams={{ page, location: 'cta', label: copy.talk }}
            >
              {copy.talk}
            </MarketingEventLink>
            <MarketingEventLink
              href={GITHUB_URL}
              target="_blank"
              className="button-dark h-12 px-6"
              event="github_click"
              eventParams={{ page, location: 'cta', label: copy.source }}
            >
              {copy.source}
            </MarketingEventLink>
            <MarketingEventLink
              href={DOCS_URL}
              target="_blank"
              className="cta-text-link"
              event="docs_click"
              eventParams={{ page, location: 'cta', label: copy.docs }}
            >
              {copy.docs}
            </MarketingEventLink>
          </div>
        </div>
      </div>
    </section>
  );
}
