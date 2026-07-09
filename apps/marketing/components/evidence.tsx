import { GITHUB_URL } from '@/lib/github';

const ACTION_ID = '0195f2a4-7c31-7a4e-a50e-2d36fb38ec42';
const DISPLAY_ACTION_ID = '0195f2a4…38ec42';
const DECISION_FIELDS = [
  ['decision', 'hold'],
  ['authority', 'passed'],
  ['risk', 'amount_above_threshold'],
  ['approval', 'required'],
  ['execution', 'not_started'],
] as const;

export function Evidence() {
  return (
    <section aria-labelledby="evidence-heading" className="section evidence-section">
      <div className="section-heading split-heading">
        <div>
          <p className="eyebrow">The evidence</p>
          <h2 id="evidence-heading" className="section-title">
            Every authorization leaves a record.
          </h2>
        </div>
        <p className="section-copy">
          See what was requested, what passed, why approval is needed, and whether execution started.
        </p>
      </div>

      <div className="evidence-grid">
        <article className="trace-sheet" aria-label="Example financial action decision receipt">
          <header>
            <div>
              <span>Example decision receipt</span>
              <code title={ACTION_ID}>{DISPLAY_ACTION_ID}</code>
            </div>
            <span className="record-state record-state-held">Held</span>
          </header>

          <div className="trace-event">
            <div className="trace-event-title">
              <span className="trace-dot" aria-hidden="true" />
              <div><small>FinancialAction</small><strong>refund · issue_refund</strong></div>
            </div>
            <dl>
              <div><dt>principal_id</dt><dd>refund-bot</dd></div>
              <div><dt>amount</dt><dd>$75.00 USD</dd></div>
              <div><dt>status</dt><dd>held</dd></div>
            </dl>
          </div>

          <dl className="decision-fields">
            {DECISION_FIELDS.map(([field, value]) => (
              <div key={field}>
                <dt>{field}</dt>
                <dd className={field === 'decision' ? 'verdict-held' : undefined}>{value}</dd>
              </div>
            ))}
          </dl>

          <footer>
            <span title="financial_action_decision_receipt.v1">decision_receipt.v1</span>
            <a href={`${GITHUB_URL}/blob/main/crates/tl-core/src/financial.rs`} target="_blank" rel="noreferrer">
              View types ↗
            </a>
          </footer>
        </article>

        <div className="evidence-notes">
          <EvidenceNote number="01" title="Authority is checked before policy">
            Mandate scope proves what this principal may do for this specific action.
          </EvidenceNote>
          <EvidenceNote number="02" title="Execution is a separate state">
            A held action cannot execute until approval moves it to authorized.
          </EvidenceNote>
          <EvidenceNote number="03" title="Proof exists on both sides">
            A decision receipt explains authorization before execution; an execution receipt records what moved.
          </EvidenceNote>
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
      <div><h3>{title}</h3><p>{children}</p></div>
    </article>
  );
}
