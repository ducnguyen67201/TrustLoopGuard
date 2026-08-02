import { randomUUID } from 'node:crypto';

import {
  readContextualPolicies,
  runContextualAgent,
  type ContextualAgentClient,
  type ContextualAgentDependencies,
  type ContextualAgentRequest,
  type ContextualAgentResult,
  type ContextualPolicySummary,
} from './agent';
import { createContextualRuntimeClient } from './runtime-client';
import type { ContextualScenarioId } from './config';

export interface HostedContextualDemoResponse extends ContextualAgentResult {
  runtime: {
    agent: 'openai-responses';
    guard: 'featherlane-ai-rust-api';
    workspace: 'shared-contextual-demo';
    data: 'synthetic-only';
  };
}

export interface HostedContextualPolicyInventoryResponse {
  policies: ContextualPolicySummary[];
  source: 'rust';
  runtime: HostedContextualDemoResponse['runtime'];
}

export interface ContextualDemoBudget {
  tryAcquire(now?: number): boolean;
}

export interface HostedContextualDemoDependencies {
  budget?: ContextualDemoBudget;
  createClient?: () => ContextualAgentClient;
  createRequestId?: () => string;
  runAgent?: typeof runContextualAgent;
  agentDependencies?: Omit<ContextualAgentDependencies, 'client' | 'logger'>;
}

export class ContextualDemoBudgetExceededError extends Error {
  constructor() {
    super('contextual demo launch budget reached');
    this.name = 'ContextualDemoBudgetExceededError';
  }
}

export class ContextualDemoRequestBudget implements ContextualDemoBudget {
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

const PUBLIC_RUN_BUDGET = new ContextualDemoRequestBudget({
  maxRequests: 60,
  windowMs: 10 * 60 * 1_000,
});

export async function runHostedContextualDemo(
  request: ContextualAgentRequest,
  dependencies: HostedContextualDemoDependencies = {},
): Promise<HostedContextualDemoResponse> {
  const budget = dependencies.budget ?? PUBLIC_RUN_BUDGET;
  if (!budget.tryAcquire()) throw new ContextualDemoBudgetExceededError();

  const requestId = (dependencies.createRequestId ?? randomUUID)();
  const logger = {
    log(step: Parameters<NonNullable<ContextualAgentDependencies['logger']>['log']>[0]): void {
      console.info('[contextual-demo]', { requestId, step, scenarioId: request.profile.scenarioId });
    },
  };
  const client = (dependencies.createClient ?? createContextualRuntimeClient)();
  const result = await (dependencies.runAgent ?? runContextualAgent)(request, {
    client,
    logger,
    ...dependencies.agentDependencies,
  });

  return { ...result, runtime: contextualRuntime() };
}

export async function readHostedContextualDemoPolicies(
  scenarioId: ContextualScenarioId,
  dependencies: Pick<HostedContextualDemoDependencies, 'createClient'> = {},
): Promise<HostedContextualPolicyInventoryResponse> {
  const client = (dependencies.createClient ?? createContextualRuntimeClient)();
  return {
    policies: await readContextualPolicies(client, scenarioId),
    source: 'rust',
    runtime: contextualRuntime(),
  };
}

function contextualRuntime(): HostedContextualDemoResponse['runtime'] {
  return {
    agent: 'openai-responses',
    guard: 'featherlane-ai-rust-api',
    workspace: 'shared-contextual-demo',
    data: 'synthetic-only',
  };
}
