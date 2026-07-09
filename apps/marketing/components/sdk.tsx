'use client';

import { useState } from 'react';
import { CodeBlock } from './code-block';

type Mode = 'sdk' | 'proxy';

const MODES = {
  sdk: {
    label: 'SDK inline',
    summary: 'Call the decision API from your runtime',
    title: 'Put the check inside your agent loop.',
    copy: 'Submit a proposed output or action to the Rust API and handle the typed decision in your own application code.',
    facts: [
      ['Boundary', 'Before output or tool execution'],
      ['Endpoint', 'POST /v1/events'],
      ['Clients', 'TypeScript, Python, Rust'],
    ],
    footerLabel: 'POST /v1/events → Decision',
  },
  proxy: {
    label: 'Gateway proxy',
    summary: 'Protect provider traffic at the base URL',
    title: 'Or enforce at the provider gateway.',
    copy: 'Route provider-compatible traffic through the gateway. TrustLoopGuard checks input and output policy and returns a trace ID with the response.',
    facts: [
      ['Gateway', '/v1/gateway/{route}/openai'],
      ['Trace', 'X-TrustLoopGuard-Trace-Id'],
      ['Runtime', 'Rust service, self-hostable'],
    ],
    footerLabel: 'Provider response + trace ID',
  },
} as const;

const SDK_SAMPLES = {
  ts: `import { guard } from '@trustloopguard/sdk';

const protect = guard({ agentId: 'support-agent' });

const safeReply = await protect({
  input: customerMessage,
  draft: proposedReply,
});

return safeReply;`,
  python: `from trustloopguard import guard

protect = guard(agent_id="support-agent")

safe_reply = await protect(
    input=customer_message,
    draft=proposed_reply,
)

return safe_reply`,
  rust: `use tl_sdk_rust::{Client, Verdict};

let client = Client::new(&std::env::var("TRUSTLOOP_URL")?);
let decision = client.submit_event(&event).await?;

match decision.verdict {
    Verdict::Allow => execute(action).await,
    Verdict::Rewrite => use_safe_output(decision.safe_output),
    Verdict::Block => refuse(decision.reason),
    Verdict::Escalate => request_review(decision.trace_id),
}`,
} as const;

const PROXY_SAMPLES = {
  ts: `import OpenAI from 'openai';

const openai = new OpenAI({
  apiKey: process.env.TRUSTLOOP_API_KEY,
  baseURL: process.env.TRUSTLOOP_URL + '/v1/gateway/support/openai',
});

const response = await openai.chat.completions.create({
  model: 'gpt-4.1-mini',
  messages,
});`,
  python: `from openai import OpenAI

client = OpenAI(
    api_key=os.environ["TRUSTLOOP_API_KEY"],
    base_url=f'{os.environ["TRUSTLOOP_URL"]}/v1/gateway/support/openai',
)

response = client.chat.completions.create(
    model="gpt-4.1-mini",
    messages=messages,
)`,
  rust: `let gateway = format!(
    "{}/v1/gateway/support/openai/chat/completions",
    std::env::var("TRUSTLOOP_URL")?
);

let response = reqwest::Client::new()
    .post(gateway)
    .bearer_auth(std::env::var("TRUSTLOOP_API_KEY")?)
    .json(&body)
    .send()
    .await?;`,
} as const;

export function Sdk() {
  const [mode, setMode] = useState<Mode>('sdk');
  const active = MODES[mode];
  const samples = mode === 'sdk' ? SDK_SAMPLES : PROXY_SAMPLES;

  return (
    <section id="developers" aria-labelledby="developers-heading" className="developers-section">
      <div className="section">
        <div className="section-heading split-heading">
          <div>
            <p className="eyebrow">For developers</p>
            <h2 id="developers-heading" className="section-title">
              Integrate at the boundary you already own.
            </h2>
          </div>
          <p className="section-copy">
            Start with one SDK call or move enforcement to a provider-compatible gateway. Both
            paths converge on the same Rust-owned decision contract.
          </p>
        </div>

        <div className="developer-grid">
          <div className="integration-rail">
            <div className="integration-tabs" role="tablist" aria-label="Integration mode">
              {(Object.keys(MODES) as Mode[]).map((key, index) => {
                const item = MODES[key];
                const selected = mode === key;
                return (
                  <button
                    key={key}
                    type="button"
                    role="tab"
                    aria-selected={selected}
                    onClick={() => setMode(key)}
                    className={`integration-tab ${selected ? 'integration-tab-active' : ''}`}
                  >
                    <span>0{index + 1}</span>
                    <div><strong>{item.label}</strong><small>{item.summary}</small></div>
                  </button>
                );
              })}
            </div>

            <div className="integration-copy">
              <h3>{active.title}</h3>
              <p>{active.copy}</p>
              <dl>
                {active.facts.map(([label, value]) => (
                  <div key={label}><dt>{label}</dt><dd>{value}</dd></div>
                ))}
              </dl>
            </div>
          </div>

          <CodeBlock samples={samples} footerLabel={active.footerLabel} />
        </div>
      </div>
    </section>
  );
}
