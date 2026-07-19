'use client';

import type { MarketingLocale } from '@/lib/marketing-locale';
import { CodeBlock } from './code-block';

const COPY = {
  en: {
    eyebrow: 'For developers',
    heading: 'Install one package. Decorate one agent.',
    intro:
      'Keep your current model provider and reply call sites. TrustLoopGuard adds one SDK decorator at the final output boundary.',
    mode: {
      label: 'Published SDK',
      summary: 'Install one package and decorate your agent',
      title: 'Keep calling the same agent.',
      copy: 'The SDK decorates your agent once, checks every final reply through the Rust runtime, and preserves the interface your application already calls.',
      facts: [
        ['Install', 'npm install @trustloopguard/sdk'],
        ['Boundary', 'Your existing reply function'],
        ['Endpoint', 'POST /v1/events'],
      ],
      footerLabel: 'Decorated agent → safe reply',
    },
  },
  vi: {
    eyebrow: 'Dành cho nhà phát triển',
    heading: 'Cài một package. Bọc một tác nhân.',
    intro:
      'Giữ nguyên nhà cung cấp mô hình và các điểm gọi phản hồi hiện tại. TrustLoopGuard thêm một decorator SDK tại ranh giới đầu ra cuối cùng.',
    mode: {
      label: 'SDK đã phát hành',
      summary: 'Cài một package và bọc tác nhân của bạn',
      title: 'Tiếp tục gọi chính tác nhân hiện có.',
      copy: 'SDK bọc tác nhân một lần, kiểm tra mọi phản hồi cuối qua runtime Rust và giữ nguyên giao diện mà ứng dụng của bạn đang gọi.',
      facts: [
        ['Cài đặt', 'npm install @trustloopguard/sdk'],
        ['Ranh giới', 'Hàm phản hồi hiện có của bạn'],
        ['Endpoint', 'POST /v1/events'],
      ],
      footerLabel: 'Tác nhân được bọc → phản hồi an toàn',
    },
  },
} as const;

const SDK_SAMPLES = {
  ts: `import { guardAgent } from '@trustloopguard/sdk';

const agent = guardAgent(createAgent(), {
  agentId: 'support-agent',
});

return await agent.reply(customerMessage);`,
  python: `from trustloopguard import guarded

@guarded(agent_id="support-agent")
async def generate_reply(message: str) -> str:
    return await agent.reply(message)`,
  rust: `use tl_sdk_rust::{AuthorizationEffect, Client};

let client = Client::new(&std::env::var("TRUSTLOOP_URL")?);
let decision = client.submit_event(&event).await?;

match decision.effect {
    AuthorizationEffect::Permit => execute(action).await,
    AuthorizationEffect::Transform => use_safe_output(decision.transformed_value),
    AuthorizationEffect::Deny => refuse(decision.reason),
    AuthorizationEffect::RequireApproval => wait_for_approval(decision.approval),
    AuthorizationEffect::Defer => wait_for_evidence(decision.reason),
}`,
} as const;

export function Sdk({ locale = 'en' }: { locale?: MarketingLocale }) {
  const copy = COPY[locale];
  const mode = copy.mode;

  return (
    <section id="developers" aria-labelledby="developers-heading" className="developers-section">
      <div className="section">
        <div className="section-heading split-heading">
          <div>
            <p className="eyebrow">{copy.eyebrow}</p>
            <h2 id="developers-heading" className="section-title">
              {copy.heading}
            </h2>
          </div>
          <p className="section-copy">{copy.intro}</p>
        </div>

        <div className="developer-grid">
          <div className="integration-rail">
            <div className="integration-tabs">
              <div className="integration-tab integration-tab-active">
                <span>01</span>
                <div>
                  <strong>{mode.label}</strong>
                  <small>{mode.summary}</small>
                </div>
              </div>
            </div>

            <div className="integration-copy">
              <h3>{mode.title}</h3>
              <p>{mode.copy}</p>
              <dl>
                {mode.facts.map(([label, value]) => (
                  <div key={label}>
                    <dt>{label}</dt>
                    <dd>{value}</dd>
                  </div>
                ))}
              </dl>
            </div>
          </div>

          <CodeBlock samples={SDK_SAMPLES} footerLabel={mode.footerLabel} locale={locale} />
        </div>
      </div>
    </section>
  );
}
