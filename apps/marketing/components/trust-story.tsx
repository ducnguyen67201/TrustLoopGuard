import { GITHUB_URL } from '@/lib/github';
import type { MarketingLocale } from '@/lib/marketing-locale';

const COPY = {
  en: {
    eyebrow: 'Why this exists',
    title: 'Built for production reality.',
    quote:
      '“Production finds what demos miss. Put policy before action—and leave proof behind.”',
    founder: 'Founder, Featherlane AI',
    principlesLabel: 'Featherlane AI design principles',
    trustIsEvidence: 'Trust is evidence',
    reviewRepository: 'Review the repository ↗',
    principles: [
      {
        number: '01',
        title: 'Policy before action',
        body: 'Check before side effects.',
      },
      {
        number: '02',
        title: 'Explain every decision',
        body: 'Return the effect and reason.',
      },
      {
        number: '03',
        title: 'Inspectable by default',
        body: 'Review the open-source runtime.',
      },
    ],
  },
  vi: {
    eyebrow: 'Vì sao sản phẩm này tồn tại',
    title: 'Được xây dựng cho thực tế production.',
    quote:
      '“Production tìm ra điều bản demo bỏ sót. Đặt chính sách trước hành động—và luôn để lại bằng chứng.”',
    founder: 'Nhà sáng lập, Featherlane AI',
    principlesLabel: 'Nguyên tắc thiết kế của Featherlane AI',
    trustIsEvidence: 'Niềm tin cần bằng chứng',
    reviewRepository: 'Xem kho mã nguồn ↗',
    principles: [
      {
        number: '01',
        title: 'Chính sách trước hành động',
        body: 'Kiểm tra trước tác dụng phụ.',
      },
      {
        number: '02',
        title: 'Giải thích mọi quyết định',
        body: 'Trả về hiệu lực và lý do.',
      },
      {
        number: '03',
        title: 'Mặc định có thể kiểm chứng',
        body: 'Xem xét runtime mã nguồn mở.',
      },
    ],
  },
} as const;

export function TrustStory({ locale = 'en' }: { locale?: MarketingLocale }) {
  const copy = COPY[locale];

  return (
    <section id="trust" aria-labelledby="trust-heading" className="section trust-section">
      <div className="section-heading trust-heading">
        <p className="eyebrow">{copy.eyebrow}</p>
        <h2 id="trust-heading" className="section-title">
          {copy.title}
        </h2>
      </div>

      <div className="trust-grid">
        <figure className="founder-note">
          <blockquote>{copy.quote}</blockquote>
          <figcaption>
            <span className="founder-avatar" aria-hidden="true">
              D
            </span>
            <span>
              <strong>Duc</strong>
              <small>{copy.founder}</small>
            </span>
          </figcaption>
        </figure>

        <div className="principles" aria-label={copy.principlesLabel}>
          <div className="principles-intro">
            <span>{copy.trustIsEvidence}</span>
            <a href={GITHUB_URL} target="_blank" rel="noreferrer">
              {copy.reviewRepository}
            </a>
          </div>
          {copy.principles.map((principle) => (
            <article key={principle.number} className="principle">
              <span>{principle.number}</span>
              <div>
                <h3>{principle.title}</h3>
                <p>{principle.body}</p>
              </div>
            </article>
          ))}
        </div>
      </div>
    </section>
  );
}
