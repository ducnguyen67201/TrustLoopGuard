import { Agent, Runner, tool, type RunContext } from '@openai/agents';
import type { AuthorizationDecision, Client } from '@trustloopguard/sdk';
import { z } from 'zod';

import { OPENAI_API_KEY, OPENAI_MODEL } from '../shared/env';
import {
  findProcurementQuote,
  procurementQuotes,
  PROCUREMENT_QUOTE_IDS,
  publicQuote,
  type ProcurementQuoteId,
  type PublicProcurementQuote,
} from './fixtures';

const PURCHASE_ORDER_SCHEMA_HASH = 'tlg-schema:procurement-submit-po-v1';
const MAX_FINAL_MESSAGE_LENGTH = 2_000;

const searchCatalogInputSchema = z.object({
  query: z.string().trim().min(1).max(120),
});

const submitPurchaseOrderInputSchema = z.object({
  quoteId: z.enum(PROCUREMENT_QUOTE_IDS),
});

const purchaseParametersSchema = z.object({
  quote_id: z.enum(PROCUREMENT_QUOTE_IDS),
  supplier_id: z.string().min(1).max(100),
  supplier_status: z.enum(['approved', 'unapproved']),
  category: z.enum(['office_furniture', 'laptops', 'office_supplies', 'gift_cards']),
  item_name: z.string().min(1).max(200),
  quantity: z.number().int().positive(),
  unit_price_minor: z.number().int().nonnegative(),
  total_minor: z.number().int().nonnegative(),
  currency: z.literal('USD'),
  review_tier: z.enum(['standard', 'high_value']),
});

export type ProcurementToolName = 'search_catalog' | 'submit_purchase_order';
export type ProcurementRunStep =
  | 'chat_received'
  | 'catalog_search'
  | 'authorization_requested'
  | 'purchase_submitted'
  | 'agent_finished';

export interface ProcurementRunLogger {
  log(step: ProcurementRunStep): void;
}

export interface ProcurementToolTrace {
  tool: ProcurementToolName;
  summary: string;
}

export interface SimulatedPurchaseOrder {
  id: string;
  quoteId: ProcurementQuoteId;
  supplierName: string;
  itemName: string;
  quantity: number;
  totalMinor: number;
  currency: 'USD';
  status: 'submitted';
}

export interface PublicAuthorizationFinding {
  id: string;
  effect: AuthorizationDecision['effect'];
  reason: string;
  severity: AuthorizationDecision['findings'][number]['severity'];
  policyId?: string;
}

export interface PublicAuthorizationDecision {
  traceId: string;
  effect: AuthorizationDecision['effect'];
  reason: string;
  latencyMs: number;
  findings: PublicAuthorizationFinding[];
  approvalId?: string;
}

export type ProcurementAuthorizationClient = Pick<Client, 'withAuthorizedAction'>;

export interface ProcurementRunContext {
  client: ProcurementAuthorizationClient;
  agentId: string;
  requestId: string;
  logger: ProcurementRunLogger;
  traces: ProcurementToolTrace[];
  purchaseOrders: SimulatedPurchaseOrder[];
  decision?: PublicAuthorizationDecision;
  authorizationAttempted: boolean;
  nextInvocationSequence: number;
}

export interface ProcurementAgentResult {
  finalMessage: string;
  traces: ProcurementToolTrace[];
  decision?: PublicAuthorizationDecision;
  purchaseOrders: SimulatedPurchaseOrder[];
}

interface ProcurementAgentDependencies {
  apiKey?: string;
  run?: (
    agent: Agent<ProcurementRunContext>,
    prompt: string,
    context: ProcurementRunContext,
  ) => Promise<string>;
}

export class ProcurementLiveAgentError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'ProcurementLiveAgentError';
  }
}

export function searchProcurementCatalog(
  query: string,
  context: ProcurementRunContext,
): readonly PublicProcurementQuote[] {
  context.logger.log('catalog_search');
  const terms = query
    .toLowerCase()
    .split(/\s+/)
    .map((term) => term.replace(/[^a-z0-9_]/g, ''))
    .filter((term) => term.length > 2);
  const allQuotes = procurementQuotes();
  const matches = allQuotes.filter((quote) => {
    const searchable = `${quote.itemName} ${quote.supplierName} ${quote.quoteId}`.toLowerCase();
    return terms.some((term) => searchable.includes(term));
  });
  const results = matches.length > 0 ? matches : allQuotes;
  context.traces.push({
    tool: 'search_catalog',
    summary: `Found ${results.length} demo quote${results.length === 1 ? '' : 's'} with server-owned pricing.`,
  });
  return results;
}

export async function submitProcurementPurchaseOrder(
  quoteId: ProcurementQuoteId,
  context: ProcurementRunContext,
): Promise<{
  status: 'submitted' | 'stopped';
  effect: AuthorizationDecision['effect'];
  reason: string;
  approvalId?: string;
  purchaseOrderId?: string;
}> {
  if (context.authorizationAttempted) {
    return {
      status: 'stopped',
      effect: context.decision?.effect ?? 'defer',
      reason: 'Only one purchase-order authorization attempt is allowed per demo run.',
    };
  }
  context.authorizationAttempted = true;
  context.nextInvocationSequence += 1;

  const quote = findProcurementQuote(quoteId);
  const parameters = {
    quote_id: quote.quoteId,
    supplier_id: quote.supplierId,
    supplier_status: quote.supplierStatus,
    category: quote.category,
    item_name: quote.itemName,
    quantity: quote.quantity,
    unit_price_minor: quote.unitPriceMinor,
    total_minor: quote.totalMinor,
    currency: quote.currency,
    review_tier: quote.reviewTier,
  };

  context.logger.log('authorization_requested');
  const result = await context.client.withAuthorizedAction(
    {
      agentId: context.agentId,
      operation: 'submit_purchase_order',
      parameters,
      invocationId: `${context.requestId}:purchase:${context.nextInvocationSequence}`,
      sideEffect: 'api_mutation',
      toolIdentity: {
        server_id: 'openai-agents',
        tool_name: 'submit_purchase_order',
        schema_hash: PURCHASE_ORDER_SCHEMA_HASH,
      },
      context: { domain: 'procurement', channel: 'chat', demo: true },
      timeoutMs: 0,
    },
    async (approvedParameters) => {
      const approved = purchaseParametersSchema.parse(approvedParameters);
      const approvedQuote = findProcurementQuote(approved.quote_id);
      const purchaseOrder: SimulatedPurchaseOrder = {
        id: `po-${context.requestId}`,
        quoteId: approved.quote_id,
        supplierName: approvedQuote.supplierName,
        itemName: approved.item_name,
        quantity: approved.quantity,
        totalMinor: approved.total_minor,
        currency: approved.currency,
        status: 'submitted',
      };
      context.purchaseOrders.push(purchaseOrder);
      context.logger.log('purchase_submitted');
      return purchaseOrder;
    },
  );

  const decision = toPublicAuthorizationDecision(result.decision);
  context.decision = decision;
  const summary = result.executed
    ? `Purchase order submitted after TrustLoopGuard returned ${decision.effect}.`
    : `Purchase order not submitted because TrustLoopGuard returned ${decision.effect}: ${decision.reason}`;
  context.traces.push({ tool: 'submit_purchase_order', summary });

  return {
    status: result.executed ? 'submitted' : 'stopped',
    effect: decision.effect,
    reason: decision.reason,
    ...(decision.approvalId === undefined ? {} : { approvalId: decision.approvalId }),
    ...(result.value === undefined ? {} : { purchaseOrderId: result.value.id }),
  };
}

const searchCatalogTool = tool<typeof searchCatalogInputSchema, ProcurementRunContext>({
  name: 'search_catalog',
  description:
    'Search the fixed demo procurement catalog. Use this before proposing a purchase order.',
  parameters: searchCatalogInputSchema,
  execute: ({ query }, runContext) => {
    const context = requireProcurementContext(runContext);
    return searchProcurementCatalog(query, context);
  },
});

const submitPurchaseOrderTool = tool<typeof submitPurchaseOrderInputSchema, ProcurementRunContext>({
  name: 'submit_purchase_order',
  description:
    'Submit one purchase order from a quote ID returned by search_catalog. TrustLoopGuard evaluates the canonical quote before execution.',
  parameters: submitPurchaseOrderInputSchema,
  execute: ({ quoteId }, runContext) => {
    const context = requireProcurementContext(runContext);
    return submitProcurementPurchaseOrder(quoteId, context);
  },
});

const PROCUREMENT_AGENT = new Agent<ProcurementRunContext>({
  name: 'Secure procurement agent',
  instructions: [
    'You are a procurement agent operating on a small demonstration catalog.',
    'Search the catalog before proposing any purchase.',
    'Use only quote IDs returned by search_catalog and never invent prices, suppliers, or availability.',
    'Call submit_purchase_order only when the buyer clearly asks to order an item.',
    'Never claim that a purchase order executed unless the tool result status is submitted.',
    'If TrustLoopGuard blocks or holds an action, explain that outcome faithfully and do not retry it.',
    'For general questions, answer briefly without proposing an action.',
    "Reply in the same language as the buyer's request.",
  ].join(' '),
  model: OPENAI_MODEL,
  modelSettings: { parallelToolCalls: false },
  tools: [searchCatalogTool, submitPurchaseOrderTool],
});

const PROCUREMENT_RUNNER = new Runner({ tracingDisabled: true });

export async function runProcurementAgent(
  prompt: string,
  context: ProcurementRunContext,
  dependencies: ProcurementAgentDependencies = {},
): Promise<ProcurementAgentResult> {
  if ((dependencies.apiKey ?? OPENAI_API_KEY)?.trim() === '') {
    throw new ProcurementLiveAgentError(
      'OPENAI_API_KEY is required for the live procurement agent',
    );
  }
  if ((dependencies.apiKey ?? OPENAI_API_KEY) === undefined) {
    throw new ProcurementLiveAgentError(
      'OPENAI_API_KEY is required for the live procurement agent',
    );
  }

  const finalOutput = dependencies.run
    ? await dependencies.run(PROCUREMENT_AGENT, prompt, context)
    : await runWithOpenAi(prompt, context);
  if (typeof finalOutput !== 'string' || finalOutput.trim() === '') {
    throw new ProcurementLiveAgentError('The live procurement agent returned no final message');
  }

  return {
    finalMessage: finalOutput.trim().slice(0, MAX_FINAL_MESSAGE_LENGTH),
    traces: context.traces.slice(0, 12),
    ...(context.decision === undefined ? {} : { decision: context.decision }),
    purchaseOrders: context.purchaseOrders.slice(0, 1),
  };
}

async function runWithOpenAi(prompt: string, context: ProcurementRunContext): Promise<string> {
  const result = await PROCUREMENT_RUNNER.run(PROCUREMENT_AGENT, prompt, {
    context,
    maxTurns: 6,
    toolExecution: { maxFunctionToolConcurrency: 1 },
  });
  if (typeof result.finalOutput !== 'string') {
    throw new ProcurementLiveAgentError('The live procurement agent returned no final message');
  }
  return result.finalOutput;
}

function requireProcurementContext(
  runContext: RunContext<ProcurementRunContext> | undefined,
): ProcurementRunContext {
  if (runContext === undefined) {
    throw new ProcurementLiveAgentError('Procurement tool context is unavailable');
  }
  return runContext.context;
}

function toPublicAuthorizationDecision(
  decision: AuthorizationDecision,
): PublicAuthorizationDecision {
  const latency = Number(decision.latency_ms);
  return {
    traceId: decision.trace_id.slice(0, 200),
    effect: decision.effect,
    reason: decision.reason.slice(0, 1_000),
    latencyMs: Number.isSafeInteger(latency) && latency >= 0 ? latency : Number.MAX_SAFE_INTEGER,
    findings: decision.findings.slice(0, 5).map((finding) => ({
      id: finding.id,
      effect: finding.effect,
      reason: finding.reason.slice(0, 1_000),
      severity: finding.severity,
      ...(finding.policy_id === undefined ? {} : { policyId: finding.policy_id }),
    })),
    ...(decision.approval === undefined ? {} : { approvalId: decision.approval.id }),
  };
}

export function quoteForPublicDisplay(quoteId: ProcurementQuoteId): PublicProcurementQuote {
  return publicQuote(findProcurementQuote(quoteId));
}
