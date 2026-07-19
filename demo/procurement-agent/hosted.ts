import { randomUUID } from 'node:crypto';

import type { Client, PolicySummary, Severity } from '@trustloopguard/sdk';

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

export interface HostedProcurementPolicyInventoryItem {
  id: ProcurementPolicyId;
  description?: string;
  severity: Severity;
  action?: string;
  enabled: boolean;
}

export type HostedProcurementPolicyInventoryResponse =
  | {
      policies: Array<HostedProcurementPolicyInventoryItem & { enabled: true }>;
      source: 'rust';
      runtime: HostedProcurementDemoResponse['runtime'];
    }
  | {
      policies: Array<HostedProcurementPolicyInventoryItem & { enabled: false }>;
      source: 'demo_template';
      runtime: HostedProcurementDemoResponse['runtime'];
    };

export interface ProcurementPolicyInventoryClient {
  listPolicies: Client['listPolicies'];
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
    runtime: procurementRuntime(),
  };
}

export async function readHostedProcurementDemoPolicies(
  dependencies: { createClient?: () => ProcurementPolicyInventoryClient } = {},
): Promise<HostedProcurementPolicyInventoryResponse> {
  const client = (dependencies.createClient ?? createClient)();
  const response = await client.listPolicies({ family: 'tool' });
  return {
    policies: projectProcurementPolicies(response.policies),
    source: 'rust',
    runtime: procurementRuntime(),
  };
}

export function readHostedProcurementDemoPolicyPreview(): HostedProcurementPolicyInventoryResponse {
  return {
    policies: PROCUREMENT_POLICIES.map((policy) => ({
      id: policy.id,
      description: policy.description,
      severity: policy.effect === 'require_approval' ? 'high' : 'critical',
      action: policy.effect,
      enabled: false,
    })),
    source: 'demo_template',
    runtime: procurementRuntime(),
  };
}

function projectProcurementPolicies(
  policies: PolicySummary[],
): Array<HostedProcurementPolicyInventoryItem & { enabled: true }> {
  const policiesById = new Map(
    policies
      .filter((policy) => policy.family === 'tool' && policy.enabled)
      .map((policy) => [policy.id, policy]),
  );

  return PROCUREMENT_POLICIES.flatMap((definition) => {
    const policy = policiesById.get(definition.id);
    if (policy === undefined) return [];
    return [
      {
        id: definition.id,
        ...(policy.description === undefined
          ? {}
          : { description: policy.description.slice(0, 300) }),
        severity: policy.severity,
        ...(policy.action === undefined ? {} : { action: policy.action.slice(0, 100) }),
        enabled: true as const,
      },
    ];
  });
}

function procurementRuntime(): HostedProcurementDemoResponse['runtime'] {
  return {
    agent: 'openai-agents-js',
    guard: 'trustloopguard-rust-api',
    provider: 'simulated-procurement-api',
  };
}
