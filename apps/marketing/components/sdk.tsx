'use client';

import { CodeBlock } from './code-block';

const SDK_MODE = {
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

export function Sdk() {
  return (
    <section id="developers" aria-labelledby="developers-heading" className="developers-section">
      <div className="section">
        <div className="section-heading split-heading">
          <div>
            <p className="eyebrow">For developers</p>
            <h2 id="developers-heading" className="section-title">
              Install one package. Decorate one agent.
            </h2>
          </div>
          <p className="section-copy">
            Keep your current model provider and reply call sites. TrustLoopGuard adds one SDK
            decorator at the final output boundary.
          </p>
        </div>

        <div className="developer-grid">
          <div className="integration-rail">
            <div className="integration-tabs">
              <div className="integration-tab integration-tab-active">
                <span>01</span>
                <div>
                  <strong>{SDK_MODE.label}</strong>
                  <small>{SDK_MODE.summary}</small>
                </div>
              </div>
            </div>

            <div className="integration-copy">
              <h3>{SDK_MODE.title}</h3>
              <p>{SDK_MODE.copy}</p>
              <dl>
                {SDK_MODE.facts.map(([label, value]) => (
                  <div key={label}>
                    <dt>{label}</dt>
                    <dd>{value}</dd>
                  </div>
                ))}
              </dl>
            </div>
          </div>

          <CodeBlock samples={SDK_SAMPLES} footerLabel={SDK_MODE.footerLabel} />
        </div>
      </div>
    </section>
  );
}
