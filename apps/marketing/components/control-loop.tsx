import type { ReactNode } from 'react';
import type { MarketingLocale } from '@/lib/marketing-locale';

const COPY = {
  en: {
    eyebrow: 'The approval journey',
    title: 'From request to execution.',
    intro: 'Policy approves, routes, or denies every action before it runs.',
    proposalLabel: 'Agent proposes',
    proposalTitle: 'Capture intent.',
    boundaryLabel: 'Policy boundary',
    boundaryTitle: 'Check policy.',
    checks: ['Grant & authority', 'Trusted evidence', 'Financial policy', 'Spend window'],
    decisionLabel: 'Decision point',
    decisionTitle: 'Return a decision.',
    outcomes: [
      {
        status: 'permit',
        detail: 'Continue to execution',
        className: 'journey-outcome-authorized',
      },
      {
        status: 'require_approval',
        detail: 'Wait for approval',
        className: 'journey-outcome-held',
      },
      { status: 'deny', detail: 'Stop before execution', className: 'journey-outcome-denied' },
    ],
    executeLabel: 'Authorized only',
    executeTitle: 'Execute if permitted.',
    proofLabel: 'Evidence after action',
    proofTitle: 'Record the outcome.',
    decisionReceipt: 'Decision receipt',
    executionReceipt: 'Execution receipt',
    everyDomain: 'One boundary:',
    footnote: 'five explicit outcomes',
  },
  vi: {
    eyebrow: 'Hành trình cấp quyền',
    title: 'Từ yêu cầu đến thực thi.',
    intro: 'Chính sách phê duyệt, chuyển tiếp hoặc từ chối mọi hành động trước khi chạy.',
    proposalLabel: 'Tác nhân đề xuất',
    proposalTitle: 'Ghi nhận ý định.',
    boundaryLabel: 'Ranh giới kiểm soát',
    boundaryTitle: 'Kiểm tra chính sách.',
    checks: [
      'Ủy quyền và thẩm quyền',
      'Bằng chứng đáng tin cậy',
      'Chính sách tài chính',
      'Hạn mức theo thời gian',
    ],
    decisionLabel: 'Điểm quyết định',
    decisionTitle: 'Trả về quyết định.',
    outcomes: [
      { status: 'permit', detail: 'Tiếp tục thực thi', className: 'journey-outcome-authorized' },
      { status: 'require_approval', detail: 'Chờ phê duyệt', className: 'journey-outcome-held' },
      { status: 'deny', detail: 'Dừng trước khi thực thi', className: 'journey-outcome-denied' },
    ],
    executeLabel: 'Chỉ khi được cấp quyền',
    executeTitle: 'Thực thi khi được phép.',
    proofLabel: 'Bằng chứng sau hành động',
    proofTitle: 'Ghi lại kết quả.',
    decisionReceipt: 'Biên nhận quyết định',
    executionReceipt: 'Biên nhận thực thi',
    everyDomain: 'Một ranh giới:',
    footnote: 'năm kết quả rõ ràng',
  },
} as const;

export function ControlLoop({ locale = 'en' }: { locale?: MarketingLocale }) {
  const copy = COPY[locale];

  return (
    <section id="how" aria-labelledby="how-heading" className="journey-section">
      <div className="section journey-intro">
        <p className="eyebrow">{copy.eyebrow}</p>
        <h2 id="how-heading" className="section-title">
          {copy.title}
        </h2>
        <p className="section-copy">{copy.intro}</p>
      </div>

      <div className="journey-canvas">
        <img
          className="journey-art"
          src="/images/trustloop-authorization-journey.png"
          alt=""
          width="921"
          height="1707"
          loading="lazy"
        />

        <StoryCard className="journey-card-proposal" number="01" label={copy.proposalLabel}>
          <h3>{copy.proposalTitle}</h3>
          <dl className="journey-action">
            <div>
              <dt>action</dt>
              <dd>issue_refund</dd>
            </div>
            <div>
              <dt>amount</dt>
              <dd>$75.00 USD</dd>
            </div>
            <div>
              <dt>status</dt>
              <dd>proposed</dd>
            </div>
          </dl>
        </StoryCard>

        <StoryCard className="journey-card-checks" number="02" label={copy.boundaryLabel}>
          <h3>{copy.boundaryTitle}</h3>
          <ul className="journey-checks">
            {copy.checks.map((check) => (
              <li key={check}>
                <span aria-hidden="true">✓</span>
                {check}
              </li>
            ))}
          </ul>
        </StoryCard>

        <StoryCard className="journey-card-decision" number="03" label={copy.decisionLabel}>
          <h3>{copy.decisionTitle}</h3>
          <div className="journey-outcomes">
            {copy.outcomes.map((outcome) => (
              <div key={outcome.status} className={outcome.className}>
                <strong>{outcome.status}</strong>
                <span>{outcome.detail}</span>
              </div>
            ))}
          </div>
        </StoryCard>

        <StoryCard className="journey-card-execute" number="04" label={copy.executeLabel}>
          <h3>{copy.executeTitle}</h3>
          <code>executeAction(action.id)</code>
        </StoryCard>

        <StoryCard className="journey-card-proof" number="05" label={copy.proofLabel}>
          <h3>{copy.proofTitle}</h3>
          <div className="journey-receipts">
            <span>{copy.decisionReceipt}</span>
            <i aria-hidden="true">→</i>
            <span>{copy.executionReceipt}</span>
          </div>
        </StoryCard>
      </div>

      <p className="journey-footnote">
        <strong>{copy.everyDomain}</strong> {copy.footnote} <code>permit</code>, <code>deny</code>,{' '}
        <code>transform</code>, <code>require_approval</code>, or <code>defer</code>.
      </p>
    </section>
  );
}

function StoryCard({
  className,
  number,
  label,
  children,
}: {
  className: string;
  number: string;
  label: string;
  children: ReactNode;
}) {
  return (
    <article className={`journey-card ${className}`}>
      <div className="journey-card-label">
        <span>{number}</span>
        <small>{label}</small>
      </div>
      {children}
    </article>
  );
}
