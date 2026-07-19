import { GITHUB_URL } from '@/lib/github';
import type { MarketingLocale } from '@/lib/marketing-locale';

const COPY = {
  en: {
    eyebrow: 'Why this exists',
    title: 'Built after watching agents fail in production.',
    quote:
      '“At an AI voice-testing company, I watched production find the failures demos missed. I built TrustLoopGuard around that lesson: assume the edge case is coming, put policy before action, and leave evidence behind.”',
    founder: 'Founder, TrustLoopGuard',
    principlesLabel: 'TrustLoopGuard design principles',
    trustIsEvidence: 'Trust is evidence',
    reviewRepository: 'Review the repository ↗',
    principles: [
      {
        number: '01',
        title: 'Policy before action',
        body: 'The check runs on the proposed output or action—not after the side effect has already happened.',
      },
      {
        number: '02',
        title: 'A reason, not a mystery',
        body: 'Every decision returns the effect, the policies that fired, and a human-readable reason.',
      },
      {
        number: '03',
        title: 'Inspectable by default',
        body: 'The runtime is open source. The wire contract, policy engine, and SDK behavior are available to review.',
      },
    ],
  },
  vi: {
    eyebrow: 'Vì sao sản phẩm này tồn tại',
    title: 'Được xây dựng sau khi chứng kiến tác nhân gặp lỗi trong production.',
    quote:
      '“Tại một công ty kiểm thử giọng nói AI, tôi đã thấy môi trường production tìm ra những lỗi mà bản demo bỏ sót. Tôi xây dựng TrustLoopGuard từ bài học đó: giả định tình huống biên sẽ xảy ra, đặt chính sách trước hành động và luôn để lại bằng chứng.”',
    founder: 'Nhà sáng lập, TrustLoopGuard',
    principlesLabel: 'Nguyên tắc thiết kế của TrustLoopGuard',
    trustIsEvidence: 'Niềm tin cần bằng chứng',
    reviewRepository: 'Xem kho mã nguồn ↗',
    principles: [
      {
        number: '01',
        title: 'Chính sách trước hành động',
        body: 'Việc kiểm tra diễn ra trên đầu ra hoặc hành động được đề xuất—không phải sau khi tác dụng phụ đã xảy ra.',
      },
      {
        number: '02',
        title: 'Có lý do, không bí ẩn',
        body: 'Mỗi quyết định trả về hiệu lực, các chính sách đã kích hoạt và lý do con người có thể đọc được.',
      },
      {
        number: '03',
        title: 'Mặc định có thể kiểm chứng',
        body: 'Runtime là mã nguồn mở. Bạn có thể xem xét hợp đồng truyền dữ liệu, bộ máy chính sách và hành vi của SDK.',
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
