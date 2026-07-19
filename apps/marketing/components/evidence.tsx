import { GITHUB_URL } from '@/lib/github';
import type { MarketingLocale } from '@/lib/marketing-locale';

const ACTION_ID = '0195f2a4-7c31-7a4e-a50e-2d36fb38ec42';
const DISPLAY_ACTION_ID = '0195f2a4…38ec42';
const DECISION_FIELDS = [
  ['effect', 'require_approval'],
  ['authority', 'passed'],
  ['risk', 'amount_above_threshold'],
  ['approval', 'required'],
  ['execution', 'not_started'],
] as const;

const COPY = {
  en: {
    eyebrow: 'The evidence',
    title: 'Every authorization leaves a record.',
    intro:
      'See what was requested, what passed, why approval is needed, and whether execution started.',
    receiptLabel: 'Example financial action decision receipt',
    receiptTitle: 'Example decision receipt',
    approvalRequired: 'Approval required',
    viewTypes: 'View types ↗',
    notes: [
      {
        title: 'Authority is checked before policy',
        body: 'Grant scope proves what this principal may do for this specific action.',
      },
      {
        title: 'Execution is a separate state',
        body: 'An approval-required action cannot execute until a matching grant and current policy produce permit.',
      },
      {
        title: 'Proof exists on both sides',
        body: 'A decision receipt explains authorization before execution; an execution receipt records what moved.',
      },
    ],
  },
  vi: {
    eyebrow: 'Bằng chứng',
    title: 'Mỗi lần cấp quyền đều để lại bản ghi.',
    intro:
      'Xem yêu cầu ban đầu, bước nào đã đạt, vì sao cần phê duyệt và việc thực thi đã bắt đầu hay chưa.',
    receiptLabel: 'Ví dụ biên nhận quyết định cho hành động tài chính',
    receiptTitle: 'Biên nhận quyết định mẫu',
    approvalRequired: 'Cần phê duyệt',
    viewTypes: 'Xem kiểu dữ liệu ↗',
    notes: [
      {
        title: 'Thẩm quyền được kiểm tra trước chính sách',
        body: 'Phạm vi ủy quyền chứng minh chủ thể này được phép làm gì đối với hành động cụ thể.',
      },
      {
        title: 'Thực thi là một trạng thái riêng biệt',
        body: 'Hành động cần phê duyệt không thể thực thi cho đến khi ủy quyền phù hợp và chính sách hiện hành trả về permit.',
      },
      {
        title: 'Có bằng chứng ở cả hai phía',
        body: 'Biên nhận quyết định giải thích việc cấp quyền trước khi thực thi; biên nhận thực thi ghi lại điều đã diễn ra.',
      },
    ],
  },
} as const;

export function Evidence({ locale = 'en' }: { locale?: MarketingLocale }) {
  const copy = COPY[locale];

  return (
    <section aria-labelledby="evidence-heading" className="section evidence-section">
      <div className="section-heading split-heading">
        <div>
          <p className="eyebrow">{copy.eyebrow}</p>
          <h2 id="evidence-heading" className="section-title">
            {copy.title}
          </h2>
        </div>
        <p className="section-copy">{copy.intro}</p>
      </div>

      <div className="evidence-grid">
        <article className="trace-sheet" aria-label={copy.receiptLabel}>
          <header>
            <div>
              <span>{copy.receiptTitle}</span>
              <code title={ACTION_ID}>{DISPLAY_ACTION_ID}</code>
            </div>
            <span className="record-state record-state-held">{copy.approvalRequired}</span>
          </header>

          <div className="trace-event">
            <div className="trace-event-title">
              <span className="trace-dot" aria-hidden="true" />
              <div>
                <small>FinancialAction</small>
                <strong>refund · issue_refund</strong>
              </div>
            </div>
            <dl>
              <div>
                <dt>principal_id</dt>
                <dd>refund-bot</dd>
              </div>
              <div>
                <dt>amount</dt>
                <dd>$75.00 USD</dd>
              </div>
              <div>
                <dt>authorization_status</dt>
                <dd>pending_approval</dd>
              </div>
              <div>
                <dt>execution_status</dt>
                <dd>not_started</dd>
              </div>
            </dl>
          </div>

          <dl className="decision-fields">
            {DECISION_FIELDS.map(([field, value]) => (
              <div key={field}>
                <dt>{field}</dt>
                <dd className={field === 'effect' ? 'effect-held' : undefined}>{value}</dd>
              </div>
            ))}
          </dl>

          <footer>
            <span title="financial_action_decision_receipt.v1">decision_receipt.v1</span>
            <a
              href={`${GITHUB_URL}/blob/main/crates/tl-core/src/financial.rs`}
              target="_blank"
              rel="noreferrer"
            >
              {copy.viewTypes}
            </a>
          </footer>
        </article>

        <div className="evidence-notes">
          {copy.notes.map((note, index) => (
            <EvidenceNote key={note.title} number={`0${index + 1}`} title={note.title}>
              {note.body}
            </EvidenceNote>
          ))}
        </div>
      </div>
    </section>
  );
}

function EvidenceNote({
  number,
  title,
  children,
}: {
  number: string;
  title: string;
  children: string;
}) {
  return (
    <article>
      <span>{number}</span>
      <div>
        <h3>{title}</h3>
        <p>{children}</p>
      </div>
    </article>
  );
}
