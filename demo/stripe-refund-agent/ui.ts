import { createServer, type IncomingMessage, type ServerResponse } from 'node:http';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { createClient, SERVER_URL, WORKSPACE_ID } from '../shared/env';
import { runRefundAgent } from './agent';
import { customerBackendState } from './order-db';
import { DEMO_ORDER_ID, type AgentRunResult, type CustomerBackendState } from './types';

const DEFAULT_UI_PORT = 9310;

interface ChatRequest {
  prompt?: string;
}

interface ChatResponse {
  result: AgentRunResult;
  state: CustomerBackendState;
}

export function startRefundAgentUi(): void {
  const server = createServer((req, res) => {
    void handleRequest(req, res);
  });
  server.listen(uiPort(), '127.0.0.1', () => {
    process.stdout.write(`Stripe refund agent UI: http://127.0.0.1:${uiPort()}\n`);
  });
}

function uiPort(): number {
  const raw = process.env.STRIPE_REFUND_AGENT_UI_PORT?.trim();
  if (raw === undefined || raw === '') return DEFAULT_UI_PORT;
  const port = Number.parseInt(raw, 10);
  return Number.isInteger(port) && port > 0 && port <= 65_535 ? port : DEFAULT_UI_PORT;
}

async function handleRequest(req: IncomingMessage, res: ServerResponse): Promise<void> {
  const url = new URL(req.url ?? '/', `http://${req.headers.host ?? '127.0.0.1'}`);
  if (req.method === 'GET' && url.pathname === '/') {
    writeHtml(res, pageHtml());
    return;
  }
  if (req.method === 'GET' && url.pathname === '/state') {
    writeJson(res, 200, customerBackendState());
    return;
  }
  if (req.method === 'POST' && url.pathname === '/chat') {
    await handleChat(req, res);
    return;
  }
  writeJson(res, 404, { error: 'not found' });
}

async function handleChat(req: IncomingMessage, res: ServerResponse): Promise<void> {
  try {
    const body = (await readJson(req)) as ChatRequest;
    const prompt = body.prompt?.trim();
    if (prompt === undefined || prompt === '') {
      writeJson(res, 400, { error: 'prompt is required' });
      return;
    }
    const result = await runRefundAgent(prompt, createClient());
    const response: ChatResponse = { result, state: customerBackendState() };
    writeJson(res, 200, response);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    writeJson(res, 500, { error: message });
  }
}

function readJson(req: IncomingMessage): Promise<unknown> {
  return new Promise((resolveBody, reject) => {
    let body = '';
    req.setEncoding('utf8');
    req.on('data', (chunk) => {
      body += chunk;
    });
    req.on('end', () => {
      try {
        resolveBody(body === '' ? {} : JSON.parse(body));
      } catch (error) {
        reject(error);
      }
    });
    req.on('error', reject);
  });
}

function writeHtml(res: ServerResponse, html: string): void {
  res.writeHead(200, { 'content-type': 'text/html; charset=utf-8' });
  res.end(html);
}

function writeJson(res: ServerResponse, statusCode: number, body: object): void {
  res.writeHead(statusCode, { 'content-type': 'application/json' });
  res.end(JSON.stringify(body));
}

function pageHtml(): string {
  return `<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>Stripe Refund Agent Demo</title>
  <style>
    :root {
      color-scheme: light;
      --bg: #f7f7f4;
      --panel: #ffffff;
      --text: #20201d;
      --muted: #6f6b63;
      --line: #d9d4ca;
      --accent: #0d766e;
      --accent-strong: #0a5e58;
      --danger: #b42318;
    }
    * { box-sizing: border-box; }
    body {
      margin: 0;
      background: var(--bg);
      color: var(--text);
      font-family: Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
    }
    main {
      width: min(1180px, calc(100vw - 32px));
      margin: 24px auto;
      display: grid;
      gap: 16px;
    }
    header {
      display: flex;
      justify-content: space-between;
      gap: 16px;
      align-items: flex-end;
      border-bottom: 1px solid var(--line);
      padding-bottom: 14px;
    }
    h1 { margin: 0; font-size: 28px; line-height: 1.1; }
    h2 { margin: 0 0 12px; font-size: 16px; }
    p { margin: 6px 0 0; color: var(--muted); }
    code { font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; }
    .grid {
      display: grid;
      grid-template-columns: minmax(0, 1.25fr) minmax(360px, 0.75fr);
      gap: 16px;
    }
    .panel {
      background: var(--panel);
      border: 1px solid var(--line);
      border-radius: 8px;
      padding: 16px;
    }
    form { display: grid; gap: 10px; }
    textarea {
      width: 100%;
      min-height: 108px;
      resize: vertical;
      border: 1px solid var(--line);
      border-radius: 8px;
      padding: 12px;
      font: inherit;
      color: var(--text);
      background: #fff;
    }
    button {
      justify-self: start;
      border: 0;
      border-radius: 8px;
      background: var(--accent);
      color: #fff;
      padding: 10px 14px;
      font: inherit;
      font-weight: 700;
      cursor: pointer;
    }
    button:disabled { opacity: 0.6; cursor: wait; }
    .trace {
      display: grid;
      gap: 8px;
      margin-top: 14px;
    }
    .step {
      border: 1px solid var(--line);
      border-radius: 8px;
      padding: 10px;
      background: #fbfaf7;
    }
    .step strong { display: block; margin-bottom: 4px; }
    .result {
      margin-top: 14px;
      padding: 12px;
      border-radius: 8px;
      background: #e9f7f4;
      border: 1px solid #a9ded5;
    }
    .error {
      color: var(--danger);
      background: #fff0ee;
      border-color: #f3b5ad;
    }
    table {
      width: 100%;
      border-collapse: collapse;
      font-size: 14px;
    }
    th, td {
      border-bottom: 1px solid var(--line);
      padding: 8px 6px;
      text-align: left;
      vertical-align: top;
    }
    th { color: var(--muted); font-weight: 700; }
    .stack { display: grid; gap: 16px; }
    .small { color: var(--muted); font-size: 13px; }
    @media (max-width: 860px) {
      .grid { grid-template-columns: 1fr; }
      header { align-items: flex-start; flex-direction: column; }
    }
  </style>
</head>
<body>
  <main>
    <header>
      <div>
        <h1>Stripe Refund Agent</h1>
        <p>Chat with a refund agent that must authorize through TrustLoopGuard before execution.</p>
      </div>
      <div class="small">
        TLG: <code>${escapeHtml(SERVER_URL)}</code><br />
        Workspace: <code>${escapeHtml(WORKSPACE_ID ?? 'default')}</code>
      </div>
    </header>

    <div class="grid">
      <section class="panel">
        <h2>Agent Chat</h2>
        <form id="chat-form">
          <textarea id="prompt">Refund order ${DEMO_ORDER_ID} for $75 because damaged item.</textarea>
          <button id="send" type="submit">Run Refund Agent</button>
        </form>
        <div id="chat-output"></div>
      </section>

      <div class="stack">
        <section class="panel">
          <h2>SQLite Orders</h2>
          <div id="orders"></div>
        </section>
        <section class="panel">
          <h2>SQLite Refunds</h2>
          <div id="refunds"></div>
        </section>
      </div>
    </div>
  </main>

  <script>
    const form = document.getElementById('chat-form');
    const send = document.getElementById('send');
    const output = document.getElementById('chat-output');

    form.addEventListener('submit', async (event) => {
      event.preventDefault();
      send.disabled = true;
      output.innerHTML = '<div class="result">Running agent...</div>';
      try {
        const prompt = document.getElementById('prompt').value;
        const res = await fetch('/chat', {
          method: 'POST',
          headers: { 'content-type': 'application/json' },
          body: JSON.stringify({ prompt }),
        });
        const json = await res.json();
        if (!res.ok) throw new Error(json.error || 'request failed');
        renderChat(json.result);
        renderState(json.state);
      } catch (error) {
        output.innerHTML = '<div class="result error">' + escapeText(String(error.message || error)) + '</div>';
      } finally {
        send.disabled = false;
      }
    });

    async function loadState() {
      const res = await fetch('/state');
      renderState(await res.json());
    }

    function renderChat(result) {
      const traces = result.traces.map((trace) =>
        '<div class="step"><strong>' + escapeText(trace.tool) + '</strong>' + escapeText(trace.summary) + '</div>'
      ).join('');
      output.innerHTML =
        '<div class="trace">' + traces + '</div>' +
        '<div class="result"><strong>Agent result</strong><br />' +
        escapeText(result.finalMessage) +
        (result.actionId ? '<br /><span class="small">action_id: <code>' + escapeText(result.actionId) + '</code></span>' : '') +
        (result.receiptId ? '<br /><span class="small">receipt_id: <code>' + escapeText(result.receiptId) + '</code></span>' : '') +
        '</div>';
    }

    function renderState(state) {
      document.getElementById('orders').innerHTML = table(
        ['order', 'paid', 'refundable', 'captured'],
        state.orders.map((order) => [
          order.id,
          dollars(order.amountPaidMinor),
          dollars(order.refundableBalanceMinor),
          order.captured ? 'yes' : 'no',
        ])
      );
      document.getElementById('refunds').innerHTML = state.refunds.length === 0
        ? '<p>No refunds recorded yet.</p>'
        : table(
            ['order', 'amount', 'status', 'provider'],
            state.refunds.map((refund) => [
              refund.orderId,
              dollars(refund.amountMinor),
              refund.status,
              refund.providerReference || 'none',
            ])
          );
    }

    function table(headers, rows) {
      return '<table><thead><tr>' + headers.map((h) => '<th>' + escapeText(h) + '</th>').join('') +
        '</tr></thead><tbody>' + rows.map((row) =>
          '<tr>' + row.map((cell) => '<td>' + escapeText(String(cell)) + '</td>').join('') + '</tr>'
        ).join('') + '</tbody></table>';
    }

    function dollars(minor) {
      return '$' + (minor / 100).toFixed(2);
    }

    function escapeText(value) {
      return value.replace(/[&<>"']/g, (char) => ({
        '&': '&amp;',
        '<': '&lt;',
        '>': '&gt;',
        '"': '&quot;',
        "'": '&#39;',
      }[char]));
    }

    loadState();
  </script>
</body>
</html>`;
}

function escapeHtml(value: string): string {
  return value.replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;');
}

function isMainModule(): boolean {
  return resolve(process.argv[1] ?? '') === fileURLToPath(import.meta.url);
}

if (isMainModule()) {
  startRefundAgentUi();
}
