import { GITHUB_URL } from '@/lib/github';
import type { MarketingLocale } from '@/lib/marketing-locale';
import { MarketingEventLink } from './marketing-event-link';

const FINANCIAL_CONTRACT_URL = `${GITHUB_URL}/blob/main/docs/concept/financial-authorization.md`;

const COPY = {
  en: {
    statusLabel: 'TrustLoopGuard product status',
    status: 'Open-source control boundary',
    statusDetail: 'Proposed action in. Typed decision out. Side effect stays on your side.',
    eyebrow: 'Runtime control for production AI agents',
    title: 'Stop AI agents',
    titleDetail: 'before they send, spend, or execute.',
    introBefore:
      'TrustLoopGuard checks a proposed output or action before it becomes a real side effect. Your runtime gets',
    effects: 'permit, deny, transform, require approval, or defer',
    introAfter: '—plus a receipt showing why.',
    founderProof: 'Built by a former engineer at a company backed by',
    demo: 'Try the live refund demo',
    controlFlow: 'See the control flow',
    demoProof: 'No card. No signup. Runs against the real authorization path.',
    inspectSource: 'Inspect the source ↗',
    proofLabel: 'Inspectable product facts',
    proofPoints: [
      { label: 'Apache-2.0', detail: 'Inspect every decision path' },
      { label: 'Self-hostable', detail: 'Rust runtime in your infrastructure' },
      { label: 'TypeScript · Python · Rust', detail: 'One generated decision contract' },
      { label: 'Decision + receipt', detail: 'Authorization before, proof after' },
    ],
    previewLabel:
      'Example control boundary: refund-bot proposes a 75 dollar refund, TrustLoopGuard requires approval, and execution does not start.',
    liveBoundary: 'Live control boundary',
    proposes: 'Agent proposes',
    proposed: 'Proposed',
    checks: 'TrustLoopGuard checks',
    authority: 'Authority',
    orderEvidence: 'Order evidence',
    refundPolicy: 'Refund policy',
    pass: 'pass',
    approval: 'approval',
    effect: 'Effect',
    decisionReturned: 'Typed decision returned',
    executionNotStarted: 'Execution not started',
    receiptReserved: 'Receipt reserved',
    stopped: 'Side effect stopped at the boundary',
    noStripeCall: 'No Stripe call made',
  },
  vi: {
    statusLabel: 'Trạng thái sản phẩm TrustLoopGuard',
    status: 'Ranh giới kiểm soát mã nguồn mở',
    statusDetail:
      'Nhận hành động đề xuất. Trả về quyết định có kiểu. Tác dụng phụ vẫn do bạn kiểm soát.',
    eyebrow: 'Kiểm soát tác nhân AI trong môi trường production',
    title: 'Chặn tác nhân AI',
    titleDetail: 'trước khi chúng gửi, chi tiền hoặc thực thi.',
    introBefore:
      'TrustLoopGuard kiểm tra đầu ra hoặc hành động được đề xuất trước khi nó tạo ra tác dụng thực tế. Hệ thống của bạn nhận về',
    effects: 'cho phép, từ chối, chuyển đổi, yêu cầu phê duyệt hoặc trì hoãn',
    introAfter: '—kèm biên nhận giải thích lý do.',
    founderProof: 'Được xây dựng bởi cựu kỹ sư tại một công ty được hậu thuẫn bởi',
    demo: 'Thử bản demo hoàn tiền trực tiếp',
    controlFlow: 'Xem luồng kiểm soát',
    demoProof: 'Không cần thẻ. Không cần đăng ký. Chạy trên luồng cấp quyền thực tế.',
    inspectSource: 'Xem mã nguồn ↗',
    proofLabel: 'Thông tin sản phẩm có thể kiểm chứng',
    proofPoints: [
      { label: 'Apache-2.0', detail: 'Kiểm tra mọi đường dẫn quyết định' },
      { label: 'Tự lưu trữ', detail: 'Runtime Rust trong hạ tầng của bạn' },
      { label: 'TypeScript · Python · Rust', detail: 'Một hợp đồng quyết định được sinh tự động' },
      { label: 'Quyết định + biên nhận', detail: 'Cấp quyền trước, bằng chứng sau' },
    ],
    previewLabel:
      'Ví dụ về ranh giới kiểm soát: refund-bot đề xuất hoàn 75 đô la, TrustLoopGuard yêu cầu phê duyệt và việc thực thi chưa bắt đầu.',
    liveBoundary: 'Ranh giới kiểm soát trực tiếp',
    proposes: 'Tác nhân đề xuất',
    proposed: 'Đã đề xuất',
    checks: 'TrustLoopGuard kiểm tra',
    authority: 'Thẩm quyền',
    orderEvidence: 'Bằng chứng đơn hàng',
    refundPolicy: 'Chính sách hoàn tiền',
    pass: 'đạt',
    approval: 'cần phê duyệt',
    effect: 'Hiệu lực',
    decisionReturned: 'Đã trả về quyết định có kiểu',
    executionNotStarted: 'Chưa bắt đầu thực thi',
    receiptReserved: 'Đã dành mã biên nhận',
    stopped: 'Tác dụng phụ bị chặn tại ranh giới',
    noStripeCall: 'Không gọi Stripe',
  },
} as const;

export function Hero({ locale = 'en' }: { locale?: MarketingLocale }) {
  const copy = COPY[locale];
  const proofHrefs = [
    `${GITHUB_URL}/blob/main/LICENSE`,
    `${GITHUB_URL}#quickstart`,
    `${GITHUB_URL}#sdk-quickstarts`,
    FINANCIAL_CONTRACT_URL,
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
            {copy.introBefore} <strong>{copy.effects}</strong>
            {copy.introAfter}
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
              href="/demo"
              className="button-primary h-12 px-6"
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
              href="#how"
              className="button-secondary h-12 px-6"
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

        <ControlBoundaryPreview locale={locale} />
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

function ControlBoundaryPreview({ locale }: { locale: MarketingLocale }) {
  const copy = COPY[locale];

  return (
    <article className="control-preview" aria-label={copy.previewLabel}>
      <header className="control-preview-header">
        <div>
          <span className="control-live-dot" aria-hidden="true" />
          {copy.liveBoundary}
        </div>
        <code>POST /v1/financial/actions</code>
      </header>

      <div className="control-proposal">
        <span className="control-node-number">01</span>
        <div>
          <p>{copy.proposes}</p>
          <strong>issue_refund</strong>
          <dl>
            <div>
              <dt>principal</dt>
              <dd>refund-bot</dd>
            </div>
            <div>
              <dt>amount</dt>
              <dd>$75.00 USD</dd>
            </div>
          </dl>
        </div>
        <span className="control-proposal-state">{copy.proposed}</span>
      </div>

      <div className="control-gate">
        <div className="control-gate-rail" aria-hidden="true">
          <span />
        </div>
        <div className="control-gate-copy">
          <p>{copy.checks}</p>
          <ul>
            <li>
              <span>{copy.authority}</span>
              <strong>{copy.pass}</strong>
            </li>
            <li>
              <span>{copy.orderEvidence}</span>
              <strong>{copy.pass}</strong>
            </li>
            <li>
              <span>{copy.refundPolicy}</span>
              <strong className="control-check-held">{copy.approval}</strong>
            </li>
          </ul>
        </div>
      </div>

      <div className="control-decision">
        <div className="control-decision-stamp">
          <small>{copy.effect}</small>
          <strong>REQUIRE</strong>
          <strong>APPROVAL</strong>
        </div>
        <div className="control-decision-copy">
          <p>{copy.decisionReturned}</p>
          <code>effect: require_approval</code>
          <div>
            <span>{copy.executionNotStarted}</span>
            <span>{copy.receiptReserved}</span>
          </div>
        </div>
      </div>

      <footer className="control-preview-footer">
        <span>
          <StopIcon />
          {copy.stopped}
        </span>
        <strong>{copy.noStripeCall}</strong>
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

function StopIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 14 14" fill="none" aria-hidden="true">
      <path
        d="M4.1 1.5h5.8l2.6 2.6v5.8l-2.6 2.6H4.1L1.5 9.9V4.1l2.6-2.6Z"
        stroke="currentColor"
        strokeWidth="1.2"
      />
      <path d="M4.25 7h5.5" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round" />
    </svg>
  );
}
