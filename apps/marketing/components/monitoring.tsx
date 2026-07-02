import { Ascii } from './ascii-art';
import { Eyebrow } from './how';
import { CountUp } from './count-up';

interface Metric {
  label: string;
  value: number;
  prefix?: string;
  suffix?: string;
  decimals?: number;
}

interface Decision {
  time: string;
  merchant: string;
  amount: string;
  verdict: 'allow' | 'rewrite' | 'block' | 'escalate';
  chip: string;
  trace: string;
}

const METRICS: Metric[] = [
  { label: 'Money screened this month', value: 1.24, prefix: '$', suffix: 'M', decimals: 2 },
  { label: 'Payments capped to policy', value: 318 },
  { label: 'Charges blocked', value: 41 },
  { label: 'Escalations resolved by a human', value: 27 },
];

const DECISIONS: Decision[] = [
  {
    time: '14:32:07',
    merchant: 'Northwind Supplies',
    amount: '$4,820.00',
    verdict: 'block',
    chip: 'BLOCK',
    trace: 'tr_9f3a71',
  },
  {
    time: '14:31:58',
    merchant: 'Stripe payout · ops',
    amount: '$1,200.00',
    verdict: 'allow',
    chip: 'ALLOW',
    trace: 'tr_a14c08',
  },
  {
    time: '14:31:12',
    merchant: 'AWS invoice (capped from $6,400)',
    amount: '$5,000.00',
    verdict: 'rewrite',
    chip: 'CAP',
    trace: 'tr_c4e0b2',
  },
  {
    time: '14:30:41',
    merchant: 'Refund · order #8842',
    amount: '$640.00',
    verdict: 'escalate',
    chip: 'HOLD',
    trace: 'tr_7c4e19',
  },
  {
    time: '14:29:55',
    merchant: 'Acme travel booking',
    amount: '$312.40',
    verdict: 'allow',
    chip: 'ALLOW',
    trace: 'tr_2b8840',
  },
  {
    time: '14:28:33',
    merchant: 'Vendor wire · net-30',
    amount: '$9,300.00',
    verdict: 'escalate',
    chip: 'HOLD',
    trace: 'tr_5f7a02',
  },
  {
    time: '14:27:50',
    merchant: 'OpenAI API · usage',
    amount: '$84.20',
    verdict: 'allow',
    chip: 'ALLOW',
    trace: 'tr_1d9c33',
  },
  {
    time: '14:26:14',
    merchant: 'Unknown payee · ACH',
    amount: '$2,500.00',
    verdict: 'block',
    chip: 'BLOCK',
    trace: 'tr_b07e55',
  },
];

export function Monitoring() {
  return (
    <section id="monitoring" aria-labelledby="monitoring-heading" className="monitoring-band">
      <div className="monitoring-inner">
        <Ascii
          name="globe"
          className="ascii-faint ascii-drift absolute -bottom-8 -left-4 hidden lg:block"
        />
        <Ascii
          name="sparkB"
          className="ascii-sm ascii-drift absolute right-4 top-4 hidden md:block"
        />
        <div className="monitoring-copy">
          <Eyebrow>05 · Monitoring</Eyebrow>
          <h2 id="monitoring-heading" className="section-title max-w-[12ch]">
            See every dollar your agent moves.
          </h2>
          <p className="mt-5 max-w-md text-base leading-7 text-[var(--color-muted)]">
            Every payment, refund, and account change, with a signed trace for who, what, and why.
          </p>
          <dl className="monitoring-stats">
            {METRICS.map((metric) => (
              <div key={metric.label} className="monitoring-stat">
                <dt className="label">{metric.label}</dt>
                <dd className="value tabular-nums">
                  <CountUp
                    to={metric.value}
                    prefix={metric.prefix}
                    suffix={metric.suffix}
                    decimals={metric.decimals}
                  />
                </dd>
              </div>
            ))}
          </dl>
        </div>

        <div
          className="monitoring-visual"
          aria-label="Live decisions feed of recent agent money actions"
        >
          <div className="monitoring-chrome">
            <span className="dot" style={{ background: 'var(--color-block)' }} aria-hidden="true" />
            <span
              className="dot"
              style={{ background: 'var(--color-rewrite)' }}
              aria-hidden="true"
            />
            <span className="dot" style={{ background: 'var(--color-allow)' }} aria-hidden="true" />
            <span className="ml-2">dashboard · decisions/live</span>
            <span className="ml-auto inline-flex items-center gap-2">
              <span className="live-dot" aria-hidden="true" />
              streaming
            </span>
          </div>
          <div className="feed">
            {DECISIONS.map((d) => (
              <div key={d.trace} className="feed-row">
                <span className="time">{d.time}</span>
                <span className="who">
                  <span className="merchant">{d.merchant}</span>
                  <span className="trace">{d.trace}</span>
                </span>
                <span className="amount tabular-nums">{d.amount}</span>
                <span className={`verdict-chip verdict-chip-${d.verdict} feed-chip`}>{d.chip}</span>
              </div>
            ))}
          </div>
        </div>
      </div>
    </section>
  );
}
