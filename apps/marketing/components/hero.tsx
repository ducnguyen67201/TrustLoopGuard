import { APP_URL } from '@/lib/app-url';
import { GITHUB_URL } from '@/lib/github';
import type { MarketingLocale } from '@/lib/marketing-locale';
import { MarketingEventLink } from './marketing-event-link';

const APPROVAL_CONTRACT_URL = `${GITHUB_URL}/blob/main/docs/concept/financial-authorization.md`;

const COPY = {
  en: {
    statusLabel: 'TrustLoopGuard product status',
    status: 'Approval infrastructure for AI agents',
    statusDetail: 'Propose. Check policy. Approve. Execute.',
    eyebrow: 'Policy approvals for consequential agent actions',
    title: 'Your agent asks.',
    titleDetail: 'Policy decides.',
    intro:
      'Put a policy approval step between an AI agent and the real world. TrustLoopGuard checks each proposed action, routes exceptions to the right person, and returns a decision before anything happens.',
    coverage: 'Approve a payment, a hospital request, or any action that needs accountable sign-off.',
    founderProof: 'Built by a former engineer at a company backed by',
    demo: 'Try a live approval',
    app: 'Go to the app',
    controlFlow: 'See the control flow',
    demoProof: 'No card. No signup. Runs against the real authorization path.',
    inspectSource: 'Inspect the source ↗',
    proofLabel: 'Inspectable product facts',
    proofPoints: [
      { label: 'Apache-2.0', detail: 'Inspect every decision path' },
      { label: 'Self-hostable', detail: 'Rust runtime in your infrastructure' },
      { label: 'Policy-native', detail: 'Permit, deny, or require approval' },
      { label: 'Auditable receipts', detail: 'Who approved what—and why' },
    ],
    previewLabel:
      'Approval request for a 75 dollar refund. Authority and refund policy pass, the amount exceeds the automatic limit, and finance approval is required before execution.',
    liveBoundary: 'Live policy decision',
    quoteId: 'REQUEST / REFUND-7F3A',
    proposes: 'Agent proposes',
    proposed: 'Awaiting decision',
    actionValue: 'Action value',
    riskPrice: 'Decision',
    coverageLimit: 'Auto-approve limit',
    priceDetail: 'Finance approval required',
    boundedTerms: 'Policy evaluation',
    authorityPolicy: 'Agent authority',
    evidenceRecovery: 'Refund evidence',
    outcomeReceipt: 'Amount threshold',
    verified: 'Passed',
    reserved: 'Approval required',
    termsLocked: 'Evaluated before execution',
    authorizedToExecute: 'Held for finance approval',
    limit: 'No payment has moved',
    flowLabel: 'Action lifecycle',
    flowSteps: ['Propose', 'Evaluate', 'Approve', 'Execute'],
    coverageDisclosure:
      'Next in queue: hospital scheduling request · requires clinical operations approval.',
  },
  vi: {
    statusLabel: 'Trạng thái sản phẩm TrustLoopGuard',
    status: 'Hạ tầng phê duyệt cho tác nhân AI',
    statusDetail: 'Đề xuất. Kiểm tra chính sách. Phê duyệt. Thực thi.',
    eyebrow: 'Phê duyệt theo chính sách cho hành động quan trọng',
    title: 'Tác nhân yêu cầu.',
    titleDetail: 'Chính sách quyết định.',
    intro:
      'Đặt một bước phê duyệt theo chính sách giữa tác nhân AI và thế giới thực. TrustLoopGuard kiểm tra từng hành động, chuyển ngoại lệ đến đúng người và trả về quyết định trước khi bất kỳ điều gì xảy ra.',
    coverage:
      'Phê duyệt khoản thanh toán, yêu cầu bệnh viện hoặc bất kỳ hành động nào cần ký duyệt có trách nhiệm.',
    founderProof: 'Được xây dựng bởi cựu kỹ sư tại một công ty được hậu thuẫn bởi',
    demo: 'Thử phê duyệt trực tiếp',
    app: 'Vào ứng dụng',
    controlFlow: 'Xem luồng kiểm soát',
    demoProof: 'Không cần thẻ. Không cần đăng ký. Chạy trên luồng cấp quyền thực tế.',
    inspectSource: 'Xem mã nguồn ↗',
    proofLabel: 'Thông tin sản phẩm có thể kiểm chứng',
    proofPoints: [
      { label: 'Apache-2.0', detail: 'Kiểm tra mọi đường dẫn quyết định' },
      { label: 'Tự lưu trữ', detail: 'Runtime Rust trong hạ tầng của bạn' },
      { label: 'Theo chính sách', detail: 'Cho phép, từ chối hoặc yêu cầu phê duyệt' },
      { label: 'Biên nhận kiểm toán', detail: 'Ai phê duyệt điều gì—và vì sao' },
    ],
    previewLabel:
      'Yêu cầu phê duyệt khoản hoàn tiền 75 đô la. Thẩm quyền và chính sách hoàn tiền đạt yêu cầu, số tiền vượt giới hạn tự động và cần phê duyệt tài chính trước khi thực thi.',
    liveBoundary: 'Quyết định chính sách trực tiếp',
    quoteId: 'YÊU CẦU / REFUND-7F3A',
    proposes: 'Tác nhân đề xuất',
    proposed: 'Đang chờ quyết định',
    actionValue: 'Giá trị hành động',
    riskPrice: 'Quyết định',
    coverageLimit: 'Giới hạn tự động',
    priceDetail: 'Cần phê duyệt tài chính',
    boundedTerms: 'Đánh giá chính sách',
    authorityPolicy: 'Thẩm quyền tác nhân',
    evidenceRecovery: 'Bằng chứng hoàn tiền',
    outcomeReceipt: 'Ngưỡng số tiền',
    verified: 'Đạt',
    reserved: 'Cần phê duyệt',
    termsLocked: 'Đánh giá trước khi thực thi',
    authorizedToExecute: 'Giữ để tài chính phê duyệt',
    limit: 'Chưa có tiền được chuyển',
    flowLabel: 'Vòng đời hành động',
    flowSteps: ['Đề xuất', 'Đánh giá', 'Phê duyệt', 'Thực thi'],
    coverageDisclosure:
      'Tiếp theo: yêu cầu đặt lịch bệnh viện · cần bộ phận vận hành lâm sàng phê duyệt.',
  },
} as const;

export function Hero({ locale = 'en' }: { locale?: MarketingLocale }) {
  const copy = COPY[locale];
  const proofHrefs = [
    `${GITHUB_URL}/blob/main/LICENSE`,
    `${GITHUB_URL}#quickstart`,
    `${GITHUB_URL}#sdk-quickstarts`,
    APPROVAL_CONTRACT_URL,
  ] as const;

  return (
    <section id="product" className="hero" aria-labelledby="hero-heading">
      <div className="hero-signal" aria-label={copy.statusLabel}>
        <p>
          <span className="hero-signal-dot" aria-hidden="true" />
          {copy.status}
        </p>
        <span>{copy.statusDetail}</span>
      </div>

      <div className="hero-inner">
        <div className="hero-copy">
          <p className="eyebrow">{copy.eyebrow}</p>
          <h1 id="hero-heading" className="hero-title">
            {copy.title}
            <span>{copy.titleDetail}</span>
          </h1>
          <p className="hero-sub">
            {copy.intro} <strong>{copy.coverage}</strong>
          </p>
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
              href={locale === 'vi' ? '/vi/demo' : '/demo'}
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
              className="hero-app-link h-12"
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
            <MarketingEventLink
              href="#how"
              className="button-secondary hero-action-button h-12"
              event="landing_cta_click"
              eventParams={{
                page: locale === 'vi' ? '/vi' : '/',
                location: 'hero',
                label: copy.controlFlow,
              }}
            >
              {copy.controlFlow}
              <ArrowIcon />
            </MarketingEventLink>
          </div>
          <div className="hero-source-row">
            <span>{copy.demoProof}</span>
            <MarketingEventLink
              href={GITHUB_URL}
              target="_blank"
              className="hero-source-link"
              event="github_click"
              eventParams={{
                page: locale === 'vi' ? '/vi' : '/',
                location: 'hero',
                label: copy.inspectSource,
              }}
            >
              {copy.inspectSource}
            </MarketingEventLink>
          </div>
        </div>

        <ApprovalPreview locale={locale} />
      </div>

      <div className="proof-strip" aria-label={copy.proofLabel}>
        {copy.proofPoints.map((item, index) => (
          <a
            key={item.label}
            href={proofHrefs[index]}
            target="_blank"
            rel="noreferrer"
            className="proof-item"
          >
            <span className="proof-number">0{index + 1}</span>
            <span>
              <strong>{item.label}</strong>
              <small>{item.detail}</small>
            </span>
            <span className="proof-arrow" aria-hidden="true">
              ↗
            </span>
          </a>
        ))}
      </div>
    </section>
  );
}

function ApprovalPreview({ locale }: { locale: MarketingLocale }) {
  const copy = COPY[locale];

  return (
    <article className="control-preview" aria-label={copy.previewLabel}>
      <header className="control-preview-header">
        <div>
          <span className="control-live-dot" aria-hidden="true" />
          {copy.liveBoundary}
        </div>
        <code>{copy.quoteId}</code>
      </header>

      <div className="quote-intent">
        <span className="quote-sequence">01</span>
        <div className="quote-intent-copy">
          <p>{copy.proposes}</p>
          <strong>issue_refund</strong>
          <span className="quote-intent-meta">
            <code>refund-bot</code>
            <span aria-hidden="true">•</span>
            <code>$75.00 USD</code>
          </span>
        </div>
        <span className="quote-ready">{copy.proposed}</span>
      </div>

      <section className="quote-rate" aria-labelledby="quote-rate-heading">
        <div className="quote-rate-primary">
          <p id="quote-rate-heading">{copy.riskPrice}</p>
          <strong>REVIEW</strong>
          <span>{copy.priceDetail}</span>
        </div>
        <dl className="quote-exposure">
          <div>
            <dt>{copy.actionValue}</dt>
            <dd>$50.00</dd>
          </div>
          <div>
            <dt>{copy.coverageLimit}</dt>
            <dd>$75.00</dd>
          </div>
        </dl>
      </section>

      <section className="quote-terms">
        <header>
          <p>{copy.boundedTerms}</p>
          <code>{copy.termsLocked}</code>
        </header>
        <ul>
          <li>
            <span>{copy.authorityPolicy}</span>
            <strong>{copy.verified}</strong>
          </li>
          <li>
            <span>{copy.evidenceRecovery}</span>
            <strong>{copy.verified}</strong>
          </li>
          <li>
            <span>{copy.outcomeReceipt}</span>
            <strong>{copy.reserved}</strong>
          </li>
        </ul>
      </section>

      <div className="quote-authorization">
        <span className="quote-authorization-icon" aria-hidden="true">
          <ProceedIcon />
        </span>
        <div>
          <p>{copy.authorizedToExecute}</p>
          <code>{copy.quoteId}</code>
        </div>
        <strong>{copy.limit}</strong>
      </div>

      <footer className="quote-flow" aria-label={copy.flowLabel}>
        <ol>
          {copy.flowSteps.map((step, index) => (
            <li key={step} data-complete={index < 2}>
              <span>0{index + 1}</span>
              <strong>{step}</strong>
            </li>
          ))}
        </ol>
        <p>{copy.coverageDisclosure}</p>
      </footer>
    </article>
  );
}

function PlayIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none" aria-hidden="true">
      <path d="M5.25 3.5 12 8l-6.75 4.5v-9Z" fill="currentColor" />
    </svg>
  );
}

function ArrowIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none" aria-hidden="true">
      <path
        d="M3 8h9M8.5 4.5 12 8l-3.5 3.5"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

function ProceedIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 14 14" fill="none" aria-hidden="true">
      <circle cx="7" cy="7" r="5.5" stroke="currentColor" strokeWidth="1.2" />
      <path
        d="m4.5 7 1.65 1.65L9.75 5"
        stroke="currentColor"
        strokeWidth="1.2"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}
