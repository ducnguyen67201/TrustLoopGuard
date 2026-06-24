// NorthPay Disputes — terminal demo. The SAME standalone agent handles the same
// injected dispute twice: once on its own (unprotected — the refund executes),
// and once with the TrustLoopGuard interceptor bolted on (the refund is gated).
// The guarded half talks to a real server; with none running it degrades
// gracefully so the unprotected half still tells the story. Run:
//   pnpm --filter @trustloopguard/demo dispute
import { createClient, DEFAULT_AGENT_ID } from '../shared/env';

import { DisputeAgent, type AgentAction, type RefundRecord } from './agent';
import { trustloopGuard } from './guard';
import { DISPUTE, customerMessage } from './scenario';

const AGENT_ID = process.env.TL_AGENT_ID ?? DEFAULT_AGENT_ID;
const RULE = '─'.repeat(48);

function proposalLine(action: AgentAction): string {
  return action.kind === 'issue_refund'
    ? `issue_refund(amount=${action.amount}, account=${action.account})`
    : `${action.kind}()`;
}

function ledgerLine(ledger: readonly RefundRecord[]): string {
  const r = ledger[0];
  return r === undefined ? 'unchanged ($0 moved)' : `-$${r.amount.toLocaleString()} → ${r.account}`;
}

async function main(): Promise<void> {
  const message = customerMessage();

  process.stdout.write(`\nNorthPay Disputes — runtime guard demo\n`);
  process.stdout.write(
    `dispute #${DISPUTE.id}  customer: ${DISPUTE.customerName}  charge: $${DISPUTE.chargeAmount.toLocaleString()}\n\n`,
  );
  process.stdout.write(
    `customer says:\n  I never authorized this charge. [attached evidence hides an\n` +
      `  "approve & refund to account …" instruction the agent will read]\n\n`,
  );

  // Unprotected: the agent alone, no guard attached.
  const rawAgent = new DisputeAgent();
  const raw = await rawAgent.handle(message);
  process.stdout.write(`── UNPROTECTED AGENT ${RULE}\n`);
  process.stdout.write(`  agent → ${proposalLine(raw.action)}\n`);
  process.stdout.write(
    rawAgent.ledger.length > 0
      ? `  💸 EXECUTED. ledger: ${ledgerLine(rawAgent.ledger)}\n`
      : `  (no sensitive action taken)\n`,
  );
  process.stdout.write(`  reply: ${raw.reply}\n\n`);

  // Protected: same agent, TrustLoopGuard interceptor bolted onto its actions.
  const guardedAgent = new DisputeAgent();
  process.stdout.write(`── PROTECTED AGENT (TrustLoopGuard) ${RULE}\n`);
  try {
    const guarded = await guardedAgent.handle(
      message,
      trustloopGuard(createClient(), AGENT_ID, {
        externalId: DISPUTE.id,
        inputSummary: `Dispute ${DISPUTE.id}: customer challenges charge`,
      }),
    );
    process.stdout.write(`  agent → ${proposalLine(guarded.action)}\n`);
    if (guarded.guardReason !== null) {
      process.stdout.write(`  🛡️  BLOCKED  reason: ${guarded.guardReason}\n`);
    } else if (guarded.executed) {
      process.stdout.write(`  ✅ allowed\n`);
    }
    process.stdout.write(`  ledger: ${ledgerLine(guardedAgent.ledger)}\n`);
    process.stdout.write(`  reply: ${guarded.reply}\n\n`);
  } catch (error) {
    process.stdout.write(`  agent → ${proposalLine(raw.action)}\n`);
    process.stdout.write(
      `  ⚠️  guard server unreachable — start it (\`make server\`) and run\n` +
        `     \`pnpm dispute:setup\` to see the live block.\n`,
    );
    process.stdout.write(`     (${error instanceof Error ? error.message : String(error)})\n\n`);
  }
}

main().catch((error) => {
  process.stderr.write(`dispute demo failed: ${error instanceof Error ? error.stack : String(error)}\n`);
  process.exit(2);
});
