import { randomUUID } from 'node:crypto';

import { createClient } from '../shared/env';
import {
  healthcarePolicyPreview,
  readHealthcarePolicies,
  runHealthcareAgent,
  type HealthcareAgentClient,
  type HealthcareAgentDependencies,
  type HealthcareAgentRequest,
  type HealthcareAgentResult,
  type HealthcarePolicyPreview,
  type HealthcarePolicySummary,
} from './agent';

export interface HostedHealthcareDemoResponse extends HealthcareAgentResult {
  runtime: {
    agent: 'openai-responses';
    guard: 'trustloopguard-rust-api';
    data: 'synthetic-only';
  };
}

export type HostedHealthcarePolicyInventoryResponse =
  | {
      policies: HealthcarePolicySummary[];
      source: 'rust';
      runtime: HostedHealthcareDemoResponse['runtime'];
    }
  | {
      policies: HealthcarePolicyPreview[];
      source: 'demo_template';
      runtime: HostedHealthcareDemoResponse['runtime'];
    };

export interface HealthcareDemoBudget {
  tryAcquire(now?: number): boolean;
}

export interface HostedHealthcareDemoDependencies {
  budget?: HealthcareDemoBudget;
  createClient?: () => HealthcareAgentClient;
  createRequestId?: () => string;
  runAgent?: typeof runHealthcareAgent;
  agentDependencies?: Omit<HealthcareAgentDependencies, 'client' | 'logger'>;
}

export class HealthcareDemoBudgetExceededError extends Error {
  constructor() {
    super('healthcare demo launch budget reached');
    this.name = 'HealthcareDemoBudgetExceededError';
  }
}

export class HealthcareDemoRequestBudget implements HealthcareDemoBudget {
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

const PUBLIC_RUN_BUDGET = new HealthcareDemoRequestBudget({
  maxRequests: 60,
  windowMs: 10 * 60 * 1_000,
});

export async function runHostedHealthcareDemo(
  request: HealthcareAgentRequest,
  dependencies: HostedHealthcareDemoDependencies = {},
): Promise<HostedHealthcareDemoResponse> {
  const budget = dependencies.budget ?? PUBLIC_RUN_BUDGET;
  if (!budget.tryAcquire()) throw new HealthcareDemoBudgetExceededError();

  const requestId = (dependencies.createRequestId ?? randomUUID)();
  const logger = {
    log(step: Parameters<NonNullable<HealthcareAgentDependencies['logger']>['log']>[0]): void {
      console.info('[healthcare-demo]', { requestId, step });
    },
  };
  const client = (dependencies.createClient ?? createClient)();
  const runAgent = dependencies.runAgent ?? runHealthcareAgent;

  console.info('[healthcare-demo]', { requestId, step: 'request_received' });
  const result = await runAgent(request, {
    client,
    logger,
    ...dependencies.agentDependencies,
  });
  console.info('[healthcare-demo]', { requestId, step: 'request_finished' });

  return {
    ...result,
    runtime: healthcareRuntime(),
  };
}

export async function readHostedHealthcareDemoPolicies(
  dependencies: Pick<HostedHealthcareDemoDependencies, 'createClient'> = {},
): Promise<HostedHealthcarePolicyInventoryResponse> {
  const client = (dependencies.createClient ?? createClient)();
  return {
    policies: await readHealthcarePolicies(client),
    source: 'rust',
    runtime: healthcareRuntime(),
  };
}

export function readHostedHealthcareDemoPolicyPreview(): HostedHealthcarePolicyInventoryResponse {
  return {
    policies: healthcarePolicyPreview(),
    source: 'demo_template',
    runtime: healthcareRuntime(),
  };
}

function healthcareRuntime(): HostedHealthcareDemoResponse['runtime'] {
  return {
    agent: 'openai-responses',
    guard: 'trustloopguard-rust-api',
    data: 'synthetic-only',
  };
}
