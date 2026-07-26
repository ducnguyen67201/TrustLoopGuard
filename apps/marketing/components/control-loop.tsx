import type { ReactNode } from 'react';
import type { MarketingLocale } from '@/lib/marketing-locale';

const COPY = {
  en: {
    eyebrow: 'The approval journey',
    title: 'Every agent action earns its way to execution.',
    intro:
      'The agent makes a request. Policy can approve it automatically, route it to a named person, or deny it. Nothing crosses the execution boundary without a decision.',
    proposalLabel: 'Agent proposes',
    proposalTitle: 'The action has not happened yet.',
    proposalBody:
      'Your runtime submits the intended action and its context before any side effect reaches a user, tool, or payment rail.',
    boundaryLabel: 'Policy boundary',
    boundaryTitle: 'Policy decides who can approve what.',
    boundaryBody:
      'Checks run against durable runtime context—not just another instruction in the prompt.',
    checks: ['Grant & authority', 'Trusted evidence', 'Financial policy', 'Spend window'],
    decisionLabel: 'Decision point',
    decisionTitle: 'Automatic or human, every approval is explicit.',
    decisionBody: 'The response tells the caller exactly what may happen next—and why.',
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
    executeTitle: 'Your runtime performs the action.',
    executeBody:
      'Approval-required actions wait. Denied actions stop. Only a permitted action continues through the gate to your existing execution code.',
    proofLabel: 'Evidence after action',
    proofTitle: 'The loop closes with proof.',
    proofBody:
      'A decision receipt records the authorization before execution. An execution receipt records what actually happened after it.',
    decisionReceipt: 'Decision receipt',
    executionReceipt: 'Execution receipt',
    everyDomain: 'Every domain:',
    footnote: 'the same boundary returns',
  },
  vi: {
    eyebrow: 'Hành trình cấp quyền',
    title: 'Một hành động được đề xuất. Một đường đi được kiểm soát đến thế giới thực.',
    intro:
      'Hãy theo đường màu xanh. Tác nhân có thể đề xuất hành động, nhưng không thể vượt qua ranh giới kiểm soát cho đến khi TrustLoopGuard trả về một quyết định có thể thực thi.',
    proposalLabel: 'Tác nhân đề xuất',
    proposalTitle: 'Hành động vẫn chưa xảy ra.',
    proposalBody:
      'Hệ thống của bạn gửi hành động dự kiến và ngữ cảnh trước khi bất kỳ tác dụng phụ nào đến người dùng, công cụ hoặc kênh thanh toán.',
    boundaryLabel: 'Ranh giới kiểm soát',
    boundaryTitle: 'TrustLoopGuard giữ hành động tại cổng.',
    boundaryBody:
      'Các bước kiểm tra dựa trên ngữ cảnh bền vững của runtime—không chỉ là một chỉ dẫn khác trong prompt.',
    checks: [
      'Ủy quyền và thẩm quyền',
      'Bằng chứng đáng tin cậy',
      'Chính sách tài chính',
      'Hạn mức theo thời gian',
    ],
    decisionLabel: 'Điểm quyết định',
    decisionTitle: 'Mọi hướng xử lý đều rõ ràng.',
    decisionBody: 'Phản hồi cho bên gọi biết chính xác điều gì có thể xảy ra tiếp theo—và vì sao.',
    outcomes: [
      { status: 'permit', detail: 'Tiếp tục thực thi', className: 'journey-outcome-authorized' },
      { status: 'require_approval', detail: 'Chờ phê duyệt', className: 'journey-outcome-held' },
      { status: 'deny', detail: 'Dừng trước khi thực thi', className: 'journey-outcome-denied' },
    ],
    executeLabel: 'Chỉ khi được cấp quyền',
    executeTitle: 'Hệ thống của bạn thực hiện hành động.',
    executeBody:
      'Hành động cần phê duyệt sẽ chờ. Hành động bị từ chối sẽ dừng. Chỉ hành động được cho phép mới đi qua cổng đến mã thực thi hiện có của bạn.',
    proofLabel: 'Bằng chứng sau hành động',
    proofTitle: 'Vòng lặp khép lại bằng bằng chứng.',
    proofBody:
      'Biên nhận quyết định ghi lại việc cấp quyền trước khi thực thi. Biên nhận thực thi ghi lại điều thực sự xảy ra sau đó.',
    decisionReceipt: 'Biên nhận quyết định',
    executionReceipt: 'Biên nhận thực thi',
    everyDomain: 'Mọi lĩnh vực:',
    footnote: 'cùng một ranh giới trả về',
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
          <p>{copy.proposalBody}</p>
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
          <p>{copy.boundaryBody}</p>
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
          <p>{copy.decisionBody}</p>
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
          <p>{copy.executeBody}</p>
          <code>executeAction(action.id)</code>
        </StoryCard>

        <StoryCard className="journey-card-proof" number="05" label={copy.proofLabel}>
          <h3>{copy.proofTitle}</h3>
          <p>{copy.proofBody}</p>
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
