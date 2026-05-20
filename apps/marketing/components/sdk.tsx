'use client';

import { useState } from 'react';
import { Eyebrow } from './how';
import { CodeBlock } from './code-block';

type Mode = 'sdk' | 'proxy';

const MODES = {
  sdk: {
    eyebrow: '04. SDK quickstart',
    label: 'SDK inline',
    summary: 'check() in app',
    title: 'Use the SDK inside your agent loop.',
    copy: 'One check returns a verdict and trace before the action reaches a user.',
    facts: [
      ['Check boundary', 'Your agent loop'],
      ['SDKs', 'TypeScript, Python, Rust'],
    ],
    footerLabel: 'POST /v1/check - Decision',
  },
  proxy: {
    eyebrow: '04. Proxy quickstart',
    label: 'Proxy server',
    summary: 'gateway in front of LLMs',
    title: 'Put the proxy in front of provider calls.',
    copy: 'Route traffic through Rust and keep provider-compatible responses.',
    facts: [
      ['Gateway', '/v1/gateway/{route}/openai'],
      ['Trace', 'X-TrustLoopGuard-Trace-Id'],
    ],
    footerLabel: 'POST /v1/gateway/{route}/openai - Provider response',
  },
} as const;

const SDK_SAMPLES = {
  ts: `import { TrustLoopGuard } from '@trustloopguard/sdk';

const trustloop = new TrustLoopGuard({
  url: process.env.TRUSTLOOP_URL,
});

const decision = await trustloop.check({
  policy: 'production',
  prompt,
  proposal,
});

if (decision.verdict === 'allow') return proposal;
if (decision.verdict === 'rewrite') return decision.rewrite;
return refuse(decision.reason);`,
  python: `from trustloopguard import TrustLoopGuard

trustloop = TrustLoopGuard(url=os.environ["TRUSTLOOP_URL"])

decision = trustloop.check(
    policy="production",
    prompt=prompt,
    proposal=proposal,
)

if decision.verdict == "allow":
    return proposal
if decision.verdict == "rewrite":
    return decision.rewrite
return refuse(decision.reason)`,
  rust: `use trustloopguard::Client;

let trustloop = Client::new(std::env::var("TRUSTLOOP_URL")?);

let decision = trustloop
    .check()
    .policy("production")
    .prompt(prompt)
    .proposal(proposal)
    .send()
    .await?;

match decision.verdict {
    Verdict::Allow => Ok(proposal),
    Verdict::Rewrite => Ok(decision.rewrite),
    _ => Ok(refuse(decision.reason)),
}`,
} as const;

const PROXY_SAMPLES = {
  ts: `import OpenAI from 'openai';

const openai = new OpenAI({
  apiKey: process.env.TRUSTLOOP_API_KEY,
  baseURL: \`\${process.env.TRUSTLOOP_URL}/v1/gateway/support/openai\`,
});

const response = await openai.chat.completions.create({
  model: 'gpt-4.1-mini',
  messages,
});

return response.choices[0]?.message.content;`,
  python: `from openai import OpenAI

client = OpenAI(
    api_key=os.environ["TRUSTLOOP_API_KEY"],
    base_url=f'{os.environ["TRUSTLOOP_URL"]}/v1/gateway/support/openai',
)

response = client.chat.completions.create(
    model="gpt-4.1-mini",
    messages=messages,
)

return response.choices[0].message.content`,
  rust: `let gateway = format!(
    "{}/v1/gateway/support/openai/chat/completions",
    std::env::var("TRUSTLOOP_URL")?
);

let response = reqwest::Client::new()
    .post(gateway)
    .bearer_auth(std::env::var("TRUSTLOOP_API_KEY")?)
    .json(&body)
    .send()
    .await?;

let safe_response = response.json::<OpenAiResponse>().await?;`,
} as const;

export function Sdk() {
  const [mode, setMode] = useState<Mode>('sdk');
  const active = MODES[mode];
  const samples = mode === 'sdk' ? SDK_SAMPLES : PROXY_SAMPLES;

  return (
    <section
      id="quickstart"
      aria-labelledby="quickstart-heading"
      className="section section-compact"
    >
      <div className="section-grid quickstart-grid">
        <div>
          <div className="mb-6 grid gap-2" role="tablist" aria-label="Integration mode">
            {(Object.keys(MODES) as Mode[]).map((key) => {
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
                  <span>{key === 'sdk' ? '01' : '02'}</span>
                  <div>
                    <p>{item.label}</p>
                    <strong>{item.summary}</strong>
                  </div>
                </button>
              );
            })}
          </div>
          <Eyebrow>{active.eyebrow}</Eyebrow>
          <h2 id="quickstart-heading" className="section-title">
            {active.title}
          </h2>
          <p className="section-copy mt-4">{active.copy}</p>
          <div className="mt-5 grid gap-2 text-sm">
            {active.facts.map(([label, value]) => (
              <Fact key={label} label={label} value={value} />
            ))}
          </div>
        </div>
        <div className="grid gap-4">
          {mode === 'proxy' && <ProxyVisual />}
          <CodeBlock samples={samples} footerLabel={active.footerLabel} />
        </div>
      </div>
    </section>
  );
}

function ProxyVisual() {
  return (
    <div className="overflow-hidden border border-[var(--color-line)] bg-white">
      <img
        src="/proxy-flow-visual.png"
        alt="TrustLoopGuard proxy flow: agent app sends requests through the TrustLoopGuard proxy to an LLM provider, then receives a safe response with trace and rewrite verdict metadata."
        className="w-full"
      />
    </div>
  );
}

function Fact({ label, value }: { label: string; value: string }) {
  return (
    <div className="grid grid-cols-[8rem_1fr] gap-4 border-t border-[var(--color-line)] pt-3">
      <span className="text-[var(--color-muted)]">{label}</span>
      <span>{value}</span>
    </div>
  );
}
