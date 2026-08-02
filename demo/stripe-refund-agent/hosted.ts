import { randomUUID } from 'node:crypto';
import { rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { resolve } from 'node:path';

import { createClient } from '../shared/env';
import { runRefundAgent } from './agent';
import { RefundDemoRequestBudget } from './auth';
import type { RefundAgentClient } from './core';
import { customerBackendState } from './order-db';
import { seedLiveRefundOrder } from './seed';
import {
  readRefundDemoActionStatus,
  type RefundDemoActionStatus,
  type RefundDemoStatusClient,
} from './status';
import type {
  AgentRunLogEntry,
  AgentRunResult,
  CustomerBackendState,
} from './types';

const PUBLIC_RUN_BUDGET = new RefundDemoRequestBudget({
  maxRequests: 60,
  windowMs: 10 * 60 * 1_000,
});

export interface HostedRefundDemoResponse {
  result: AgentRunResult;
  state: CustomerBackendState;
  logs: AgentRunLogEntry[];
  runtime: {
    agent: 'openai';
    guard: 'featherlane-ai-rust-api';
    provider: 'stripe-test';
  };
}

type HostedClient = RefundAgentClient & RefundDemoStatusClient;

export interface HostedRefundDemoDependencies {
  budget?: Pick<RefundDemoRequestBudget, 'tryAcquire'>;
  createClient?: () => HostedClient;
  createRequestId?: () => string;
  temporaryDirectory?: () => string;
  seedOrder?: typeof seedLiveRefundOrder;
  runAgent?: typeof runRefundAgent;
  readState?: typeof customerBackendState;
  readStatus?: typeof readRefundDemoActionStatus;
  removeDatabase?: (dbPath: string) => void;
}

export class RefundDemoBudgetExceededError extends Error {
  constructor() {
    super('refund demo launch budget reached');
    this.name = 'RefundDemoBudgetExceededError';
  }
}

export async function runHostedRefundDemo(
  prompt: string,
  dependencies: HostedRefundDemoDependencies = {},
): Promise<HostedRefundDemoResponse> {
  const budget = dependencies.budget ?? PUBLIC_RUN_BUDGET;
  if (!budget.tryAcquire()) throw new RefundDemoBudgetExceededError();

  const requestId = (dependencies.createRequestId ?? randomUUID)();
  const dbPath = resolve(
    (dependencies.temporaryDirectory ?? tmpdir)(),
    'featherlane-ai-refund-demo',
    `${requestId}.sqlite`,
  );
  const logs: AgentRunLogEntry[] = [];
  const logger = {
    log(step: string, message: string): void {
      logs.push({ step, message });
      console.info('[refund-demo]', { requestId, step });
    },
  };

  try {
    const seedOrder = dependencies.seedOrder ?? seedLiveRefundOrder;
    const runAgent = dependencies.runAgent ?? runRefundAgent;
    const readState = dependencies.readState ?? customerBackendState;
    const client = (dependencies.createClient ?? createClient)() as HostedClient;

    logger.log('chat', 'received refund request');
    logger.log('stripe_fixture', 'creating a fresh captured $100 Stripe test order');
    await seedOrder({ dbPath });
    const result = await runAgent(prompt, client, {
      logger,
      requestId,
      requireLiveAgent: true,
      dbPath,
      allowGrantProvisioning: false,
    });
    logger.log('chat', 'refund agent finished');

    return {
      result,
      state: readState(dbPath),
      logs,
      runtime: {
        agent: 'openai',
        guard: 'featherlane-ai-rust-api',
        provider: 'stripe-test',
      },
    };
  } finally {
    (dependencies.removeDatabase ?? removeDatabase)(dbPath);
  }
}

export async function readHostedRefundDemoStatus(
  actionId: string,
  dependencies: Pick<HostedRefundDemoDependencies, 'createClient' | 'readStatus'> = {},
): Promise<RefundDemoActionStatus> {
  const client = (dependencies.createClient ?? createClient)() as HostedClient;
  return (dependencies.readStatus ?? readRefundDemoActionStatus)(client, actionId);
}

function removeDatabase(dbPath: string): void {
  rmSync(dbPath, { force: true });
}
