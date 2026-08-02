import { createInterface } from 'node:readline/promises';

import { createClient, SERVER_URL, WORKSPACE_ID } from '../shared/env';
import { runRefundAgent } from './agent';
import { DEMO_ORDER_ID } from './types';

async function main(): Promise<void> {
  const prompt = await promptFromArgsOrStdin();
  const result = await runRefundAgent(prompt, createClient());

  process.stdout.write('\nRefund agent demo\n');
  process.stdout.write(`Featherlane AI: ${SERVER_URL}\n`);
  if (WORKSPACE_ID) process.stdout.write(`Workspace: ${WORKSPACE_ID}\n`);
  process.stdout.write(`User: ${result.prompt}\n\n`);

  for (const trace of result.traces) {
    process.stdout.write(`${trace.tool} -> ${trace.summary}\n`);
  }

  process.stdout.write(`\n${result.finalMessage}\n`);
  if (result.actionId !== undefined) process.stdout.write(`action_id: ${result.actionId}\n`);
  if (result.receiptId !== undefined) process.stdout.write(`receipt_id: ${result.receiptId}\n`);
}

async function promptFromArgsOrStdin(): Promise<string> {
  const fromArgs = process.argv.slice(2).join(' ').trim();
  if (fromArgs !== '') return fromArgs;

  if (!process.stdin.isTTY) {
    const chunks: Buffer[] = [];
    for await (const chunk of process.stdin) {
      chunks.push(Buffer.isBuffer(chunk) ? chunk : Buffer.from(String(chunk)));
    }
    const piped = Buffer.concat(chunks).toString('utf8').trim();
    if (piped !== '') return piped;
  }

  process.stdout.write(`Example: Refund order ${DEMO_ORDER_ID} for $75 because damaged item.\n`);
  const rl = createInterface({ input: process.stdin, output: process.stdout });
  try {
    return (await rl.question('Ask the refund agent: ')).trim();
  } finally {
    rl.close();
  }
}

main().catch((error) => {
  const message = error instanceof Error ? error.message : String(error);
  const hint = message.includes('missing bearer token')
    ? '\nSet TL_API_KEY for this local server, then rerun the agent.'
    : '';
  process.stderr.write(`stripe refund agent failed: ${message}${hint}\n`);
  process.exitCode = 1;
});
