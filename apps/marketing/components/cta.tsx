import { BOOK_MEETING_URL, DOCS_URL, GITHUB_URL } from '@/lib/github';
import type { MarketingLocale } from '@/lib/marketing-locale';
import { MarketingEventLink } from './marketing-event-link';

const COPY = {
  en: {
    eyebrow: 'Start with one approval',
    title: 'Choose the agent action that needs a real sign-off.',
    body: 'We will map the request, the policy, the approver, and the execution boundary—whether it is a payment, a hospital workflow, or your own high-impact action.',
    talk: 'Design an approval flow',
    source: 'Review the source',
    docs: 'Read the docs ↗',
  },
  vi: {
    eyebrow: 'Bắt đầu từ một đường dẫn lỗi thực tế',
    title: 'Hãy mang đến hành động của tác nhân khiến bạn lo lắng nhất khi phát hành.',
    body: 'Chúng tôi sẽ cùng bạn xác định sự kiện, ranh giới chính sách và quyết định mà runtime cần trước khi hành động đó đến người dùng hoặc công cụ.',
    talk: 'Trao đổi về một đường dẫn lỗi',
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
