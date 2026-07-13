import { randomUUID } from 'node:crypto';
import { rmSync } from 'node:fs';
import { createServer, type IncomingMessage, type Server, type ServerResponse } from 'node:http';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { createClient, SERVER_URL, WORKSPACE_ID } from '../shared/env';
import { runRefundAgent } from './agent';
import {
  RefundDemoRequestBudget,
  isValidRefundDemoAuthorization,
  requireRefundDemoProxySecret,
} from './auth';
import { customerBackendState, orderDatabasePath } from './order-db';
import { handleProviderPayment, providerApiKey } from './provider';
import { seedLiveRefundOrder } from './seed';
import { readRefundDemoActionStatus } from './status';
import { stripeTestKeyFromEnv } from './stripe';
import {
  DEMO_ORDER_ID,
  type AgentRunLogEntry,
  type AgentRunResult,
  type CustomerBackendState,
  type StripeRefundProviderRequest,
} from './types';

const DEFAULT_UI_PORT = 9310;
const PUBLIC_RUN_BUDGET = new RefundDemoRequestBudget({
  maxRequests: 60,
  windowMs: 10 * 60 * 1_000,
});

interface ChatRequest {
  prompt?: string;
}

interface ChatResponse {
  result: AgentRunResult;
  state: CustomerBackendState;
  logs: AgentRunLogEntry[];
  runtime: {
    agent: 'openai';
    guard: 'trustloopguard-rust-api';
    provider: 'stripe-test';
  };
}

export function startRefundAgentUi(): void {
  requireRefundDemoProxySecret();
  providerApiKey();
  const server = createRefundAgentServer();
  server.listen(refundAgentPort(), refundAgentHost(), () => {
    process.stdout.write(
      `Stripe refund agent service: http://${refundAgentHost()}:${refundAgentPort()}\n`,
    );
  });
}

export function createRefundAgentServer(): Server {
  return createServer((req, res) => {
    void handleRequest(req, res);
  });
}

export function refundAgentHost(
  raw = process.env.STRIPE_REFUND_AGENT_UI_HOST,
): string {
  const host = raw?.trim() || '127.0.0.1';
  if (!['127.0.0.1', '0.0.0.0', '::1', '::'].includes(host)) {
    throw new Error('STRIPE_REFUND_AGENT_UI_HOST must be a local bind address');
  }
  return host;
}

export function refundAgentPort(
  raw = process.env.STRIPE_REFUND_AGENT_UI_PORT ?? process.env.PORT,
): number {
  raw = raw?.trim();
  if (raw === undefined || raw === '') return DEFAULT_UI_PORT;
  const port = Number.parseInt(raw, 10);
  return Number.isInteger(port) && port > 0 && port <= 65_535 ? port : DEFAULT_UI_PORT;
}

async function handleRequest(req: IncomingMessage, res: ServerResponse): Promise<void> {
  const url = new URL(req.url ?? '/', `http://${req.headers.host ?? '127.0.0.1'}`);
  if (req.method === 'GET' && url.pathname === '/health') {
    writeJson(res, 200, { status: 'ok' });
    return;
  }
  if (req.method === 'GET' && url.pathname === '/') {
    if (!isDirectLoopbackRequest(req)) {
      writeJson(res, 404, { error: 'not found' });
      return;
    }
    writeHtml(res, pageHtml(requireRefundDemoProxySecret()));
    return;
  }
  if (req.method === 'GET' && url.pathname === '/state') {
    if (!isDirectLoopbackRequest(req)) {
      writeJson(res, 404, { error: 'not found' });
      return;
    }
    writeJson(res, 200, customerBackendState());
    return;
  }
  if (req.method === 'GET' && url.pathname.startsWith('/status/')) {
    if (!authorizeRequest(req, res)) return;
    const actionId = decodeURIComponent(url.pathname.slice('/status/'.length));
    if (!/^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i.test(actionId)) {
      writeJson(res, 400, { error: 'invalid action id' });
      return;
    }
    try {
      writeJson(res, 200, await readRefundDemoActionStatus(createClient(), actionId));
    } catch (error) {
      process.stderr.write(`[stripe-refund-agent] status error: ${safeErrorForLog(error)}\n`);
      writeJson(res, 404, { error: 'action status not found' });
    }
    return;
  }
  if (req.method === 'POST' && url.pathname === '/payments') {
    try {
      const body = (await readJson(req)) as StripeRefundProviderRequest;
      const reply = await handleProviderPayment(req.headers.authorization, body);
      writeJson(res, reply.statusCode, reply.body);
    } catch (error) {
      process.stderr.write(`[stripe-refund-agent] provider error: ${safeErrorForLog(error)}\n`);
      writeJson(res, 500, { error: 'provider request failed' });
    }
    return;
  }
  if (req.method === 'POST' && url.pathname === '/reset') {
    if (!authorizeMutation(req, res)) return;
    try {
      await seedLiveRefundOrder();
      writeJson(res, 200, customerBackendState());
    } catch (error) {
      writeJson(res, 500, {
        error: error instanceof Error ? error.message : String(error),
      });
    }
    return;
  }
  if (req.method === 'POST' && url.pathname === '/chat') {
    if (!authorizeMutation(req, res)) return;
    await handleChat(req, res);
    return;
  }
  writeJson(res, 404, { error: 'not found' });
}

async function handleChat(req: IncomingMessage, res: ServerResponse): Promise<void> {
  const requestId = randomUUID();
  const dbPath = requestDatabasePath(requestId);
  const logs: AgentRunLogEntry[] = [];
  const logger = {
    log(step: string, message: string): void {
      logs.push({ step, message });
      process.stdout.write(`[stripe-refund-agent] ${requestId} ${step}: ${message}\n`);
    },
  };

  try {
    const body = (await readJson(req)) as ChatRequest;
    const prompt = body.prompt?.trim();
    if (prompt === undefined || prompt === '') {
      writeJson(res, 400, { error: 'prompt is required' });
      return;
    }
    logger.log('chat', 'received refund request');
    logger.log('stripe_fixture', 'creating a fresh captured $100 Stripe test order');
    await seedLiveRefundOrder({ dbPath });
    const result = await runRefundAgent(prompt, createClient(), {
      logger,
      requestId,
      requireLiveAgent: true,
      dbPath,
    });
    logger.log('chat', 'refund agent finished');
    const response: ChatResponse = {
      result,
      state: customerBackendState(dbPath),
      logs,
      runtime: {
        agent: 'openai',
        guard: 'trustloopguard-rust-api',
        provider: 'stripe-test',
      },
    };
    writeJson(res, 200, response);
  } catch (error) {
    process.stderr.write(`[stripe-refund-agent] ${requestId} error: ${safeErrorForLog(error)}\n`);
    writeJson(res, 500, {
      error: 'The refund workflow failed safely. No refund was executed.',
    });
  } finally {
    rmSync(dbPath, { force: true });
  }
}

function authorizeMutation(req: IncomingMessage, res: ServerResponse): boolean {
  if (!authorizeRequest(req, res)) return false;
  if (!PUBLIC_RUN_BUDGET.tryAcquire()) {
    writeJson(res, 429, { error: 'demo budget reached; try again later' });
    return false;
  }
  return true;
}

function authorizeRequest(req: IncomingMessage, res: ServerResponse): boolean {
  if (
    !isValidRefundDemoAuthorization(
      req.headers.authorization,
      requireRefundDemoProxySecret(),
    )
  ) {
    writeJson(res, 401, { error: 'unauthorized' });
    return false;
  }
  return true;
}

function isDirectLoopbackRequest(req: IncomingMessage): boolean {
  const address = req.socket.remoteAddress;
  const isLoopback =
    address === '127.0.0.1' || address === '::1' || address === '::ffff:127.0.0.1';
  return (
    isLoopback &&
    req.headers.forwarded === undefined &&
    req.headers['x-forwarded-for'] === undefined &&
    req.headers['x-real-ip'] === undefined
  );
}

function requestDatabasePath(requestId: string): string {
  return resolve(dirname(orderDatabasePath()), 'runs', `${requestId}.sqlite`);
}

function safeErrorForLog(error: unknown): string {
  if (!(error instanceof Error)) return 'unknown error';
  return `${error.name}: ${error.message}`.slice(0, 500);
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

function pageHtml(proxySecret: string): string {
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
    .actions {
      display: flex;
      flex-wrap: wrap;
      gap: 8px;
      margin-top: 12px;
    }
    .secondary {
      background: transparent;
      color: var(--text);
      border: 1px solid var(--line);
    }
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
    const proxyAuthorization = ${JSON.stringify(`Bearer ${proxySecret}`)};
    const form = document.getElementById('chat-form');
    const send = document.getElementById('send');
    const output = document.getElementById('chat-output');
    const promptInput = document.getElementById('prompt');

    form.addEventListener('submit', async (event) => {
      event.preventDefault();
      await runPrompt(promptInput.value);
    });

    output.addEventListener('click', async (event) => {
      const target = event.target;
      if (!(target instanceof HTMLElement)) return;
      const nextPrompt = target.dataset.prompt;
      if (nextPrompt) {
        promptInput.value = nextPrompt;
        await runPrompt(nextPrompt);
        return;
      }
      if (target.dataset.reset === 'true') {
        target.setAttribute('disabled', 'true');
        const res = await fetch('/reset', {
          method: 'POST',
          headers: { authorization: proxyAuthorization },
        });
        renderState(await res.json());
        output.innerHTML = '<div class="result">Seeded order reset. Try the refund again.</div>';
      }
    });

    async function runPrompt(prompt) {
      send.disabled = true;
      output.innerHTML = '<div class="result">Running agent...</div>';
      try {
        const res = await fetch('/chat', {
          method: 'POST',
          headers: {
            authorization: proxyAuthorization,
            'content-type': 'application/json',
          },
          body: JSON.stringify({ prompt }),
        });
        const json = await res.json();
        if (!res.ok) {
          renderError(json.error || 'request failed', json.logs || []);
          if (json.state) renderState(json.state);
          return;
        }
        renderChat(json.result, json.logs || [], json.state);
        renderState(json.state);
      } catch (error) {
        renderError(String(error.message || error), []);
      } finally {
        send.disabled = false;
      }
    }

    async function loadState() {
      const res = await fetch('/state');
      renderState(await res.json());
    }

    function renderChat(result, logs, state) {
      const traces = result.traces.map((trace) =>
        '<div class="step"><strong>' + escapeText(trace.tool) + '</strong>' + escapeText(trace.summary) + '</div>'
      ).join('');
      output.innerHTML =
        renderRunLogs(logs) +
        '<div class="trace">' + traces + '</div>' +
        '<div class="result"><strong>Agent result</strong><br />' +
        escapeText(result.finalMessage) +
        (result.actionId ? '<br /><span class="small">action_id: <code>' + escapeText(result.actionId) + '</code></span>' : '') +
        (result.receiptId ? '<br /><span class="small">receipt_id: <code>' + escapeText(result.receiptId) + '</code></span>' : '') +
        suggestedActions(result, state) +
        '</div>';
    }

    function suggestedActions(result, state) {
      const message = String(result.finalMessage || '').toLowerCase();
      const needsFollowUp = message.includes('would you like to proceed') || message.includes('less than the requested');
      if (!needsFollowUp || !state || !Array.isArray(state.orders)) return '';
      const orderId = result.prompt.match(/ord_[a-z0-9_]+/i)?.[0];
      const order = state.orders.find((item) => item.id === orderId) || state.orders[0];
      if (!order) return '';
      const actions = [];
      if (order.refundableBalanceMinor > 0) {
        const prompt = 'Refund order ' + order.id + ' for ' + dollars(order.refundableBalanceMinor) + ' because damaged item.';
        actions.push(
          '<button type="button" data-prompt="' + escapeAttr(prompt) + '">Ask for available ' + escapeText(dollars(order.refundableBalanceMinor)) + '</button>'
        );
      }
      actions.push('<button class="secondary" type="button" data-reset="true">Reset seeded order</button>');
      return '<div class="actions">' + actions.join('') + '</div>';
    }

    function renderError(message, logs) {
      output.innerHTML =
        renderRunLogs(logs) +
        '<div class="result error"><strong>Run failed</strong><br />' + escapeText(message) + '</div>';
    }

    function renderRunLogs(logs) {
      if (!logs.length) return '';
      return '<div class="trace">' + logs.map((entry) =>
        '<div class="step"><strong>' + escapeText(entry.step) + '</strong>' + escapeText(entry.message) + '</div>'
      ).join('') + '</div>';
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
      return String(value).replace(/[&<>"']/g, (char) => ({
        '&': '&amp;',
        '<': '&lt;',
        '>': '&gt;',
        '"': '&quot;',
        "'": '&#39;',
      }[char]));
    }

    function escapeAttr(value) {
      return escapeText(value);
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
  try {
    if (stripeTestKeyFromEnv() === null) {
      throw new Error('STRIPE_SECRET_KEY is required for the live refund demo');
    }
    startRefundAgentUi();
  } catch (error) {
    process.stderr.write(
      `Stripe refund agent UI failed: ${error instanceof Error ? error.message : String(error)}\n`,
    );
    process.exitCode = 1;
  }
}
