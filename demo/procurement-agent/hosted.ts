import { randomUUID } from 'node:crypto';

import { createClient } from '../shared/env';
import {
  runProcurementAgent,
  type ProcurementAgentResult,
  type ProcurementAuthorizationClient,
  type ProcurementRunContext,
  type ProcurementRunStep,
  type ProcurementToolTrace,
  type PublicAuthorizationDecision,
  type SimulatedPurchaseOrder,
} from './agent';
import {
  normalizeProcurementPolicyIds,
  procurementAgentId,
  PROCUREMENT_POLICIES,
  type ProcurementPolicyEffect,
  type ProcurementPolicyId,
} from './fixtures';

export interface HostedProcurementPolicy {
  id: ProcurementPolicyId;
  title: string;
  description: string;
  effect: ProcurementPolicyEffect;
  enabled: boolean;
}

export interface HostedProcurementLogEntry {
  step: ProcurementRunStep;
}

export interface HostedProcurementDemoResponse {
  result: {
    finalMessage: string;
    traces: ProcurementToolTrace[];
    decision?: PublicAuthorizationDecision;
  };
  state: {
    purchaseOrders: SimulatedPurchaseOrder[];
  };
  activePolicies: HostedProcurementPolicy[];
  logs: HostedProcurementLogEntry[];
  runtime: {
    agent: 'openai-agents-js';
    guard: 'trustloopguard-rust-api';
    provider: 'simulated-procurement-api';
  };
}

export interface HostedProcurementDemoDependencies {
  budget?: Pick<ProcurementDemoRequestBudget, 'tryAcquire'>;
  createClient?: () => ProcurementAuthorizationClient;
  createRequestId?: () => string;
  runAgent?: typeof runProcurementAgent;
}

export class ProcurementDemoRequestBudget {
  private count = 0;
  private resetAt = 0;

  constructor(
    private readonly options: {
      maxRequests: number;
      windowMs: number;
    },
  ) {}

  tryAcquire(now = Date.now()): boolean {
    if (this.resetAt <= now) {
      this.count = 0;
      this.resetAt = now + this.options.windowMs;
    }
    if (this.count >= this.options.maxRequests) return false;
    this.count += 1;
    return true;
  }
}

export class ProcurementDemoBudgetExceededError extends Error {
  constructor() {
    super('procurement demo launch budget reached');
    this.name = 'ProcurementDemoBudgetExceededError';
  }
}

const PUBLIC_RUN_BUDGET = new ProcurementDemoRequestBudget({
  maxRequests: 60,
  windowMs: 10 * 60 * 1_000,
});

export async function runHostedProcurementDemo(
  prompt: string,
  activePolicyIds: readonly ProcurementPolicyId[],
  dependencies: HostedProcurementDemoDependencies = {},
): Promise<HostedProcurementDemoResponse> {
  const budget = dependencies.budget ?? PUBLIC_RUN_BUDGET;
  if (!budget.tryAcquire()) throw new ProcurementDemoBudgetExceededError();

  const requestId = (dependencies.createRequestId ?? randomUUID)();
  const normalizedPolicyIds = normalizeProcurementPolicyIds(activePolicyIds);
  const logs: HostedProcurementLogEntry[] = [];
  const logger = {
    log(step: ProcurementRunStep): void {
      logs.push({ step });
      console.info('[procurement-demo]', { requestId, step });
    },
  };
  const context: ProcurementRunContext = {
    client: (dependencies.createClient ?? createClient)(),
    agentId: procurementAgentId(normalizedPolicyIds),
    requestId,
    logger,
    traces: [],
    purchaseOrders: [],
    authorizationAttempted: false,
    nextInvocationSequence: 0,
  };

  logger.log('chat_received');
  const runAgent = dependencies.runAgent ?? runProcurementAgent;
  const agentResult: ProcurementAgentResult = await runAgent(prompt, context);
  logger.log('agent_finished');

  return {
    result: {
      finalMessage: agentResult.finalMessage,
      traces: agentResult.traces,
      ...(agentResult.decision === undefined ? {} : { decision: agentResult.decision }),
    },
    state: { purchaseOrders: agentResult.purchaseOrders },
    activePolicies: PROCUREMENT_POLICIES.map((policy) => ({
      ...policy,
      enabled: normalizedPolicyIds.includes(policy.id),
    })),
    logs,
    runtime: {
      agent: 'openai-agents-js',
      guard: 'trustloopguard-rust-api',
      provider: 'simulated-procurement-api',
    },
  };
}
