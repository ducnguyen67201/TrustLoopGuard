import { stringify } from 'yaml';

export const PROCUREMENT_POLICY_IDS = [
  'procurement-approved-suppliers',
  'procurement-high-value-review',
  'procurement-restricted-categories',
] as const;

export type ProcurementPolicyId = (typeof PROCUREMENT_POLICY_IDS)[number];
export type ProcurementPolicyEffect = 'deny' | 'require_approval';

export interface ProcurementPolicyDefinition {
  id: ProcurementPolicyId;
  title: string;
  description: string;
  effect: ProcurementPolicyEffect;
}

export const PROCUREMENT_POLICIES: readonly ProcurementPolicyDefinition[] = [
  {
    id: 'procurement-approved-suppliers',
    title: 'Approved suppliers only',
    description: 'Blocks purchase orders for suppliers outside the approved vendor list.',
    effect: 'deny',
  },
  {
    id: 'procurement-high-value-review',
    title: 'Review high-value orders',
    description: 'Holds high-value purchase orders for an owner or administrator to approve.',
    effect: 'require_approval',
  },
  {
    id: 'procurement-restricted-categories',
    title: 'Block restricted categories',
    description: 'Blocks gift cards and other categories that procurement does not permit.',
    effect: 'deny',
  },
] as const;

export const PROCUREMENT_QUOTE_IDS = [
  'quote-approved-chairs',
  'quote-high-value-laptops',
  'quote-unapproved-supplies',
  'quote-restricted-gift-cards',
] as const;

export type ProcurementQuoteId = (typeof PROCUREMENT_QUOTE_IDS)[number];
export type SupplierStatus = 'approved' | 'unapproved';
export type ProcurementCategory = 'office_furniture' | 'laptops' | 'office_supplies' | 'gift_cards';
export type ProcurementReviewTier = 'standard' | 'high_value';

export interface ProcurementQuote {
  quoteId: ProcurementQuoteId;
  supplierId: string;
  supplierName: string;
  supplierStatus: SupplierStatus;
  category: ProcurementCategory;
  itemName: string;
  quantity: number;
  unitPriceMinor: number;
  totalMinor: number;
  currency: 'USD';
  reviewTier: ProcurementReviewTier;
}

const QUOTES: Readonly<Record<ProcurementQuoteId, Readonly<ProcurementQuote>>> = {
  'quote-approved-chairs': Object.freeze({
    quoteId: 'quote-approved-chairs',
    supplierId: 'supplier-northstar-office',
    supplierName: 'Northstar Office',
    supplierStatus: 'approved',
    category: 'office_furniture',
    itemName: 'Ergonomic office chairs',
    quantity: 20,
    unitPriceMinor: 12_000,
    totalMinor: 240_000,
    currency: 'USD',
    reviewTier: 'standard',
  }),
  'quote-high-value-laptops': Object.freeze({
    quoteId: 'quote-high-value-laptops',
    supplierId: 'supplier-apex-computing',
    supplierName: 'Apex Computing',
    supplierStatus: 'approved',
    category: 'laptops',
    itemName: 'Developer laptops',
    quantity: 50,
    unitPriceMinor: 84_000,
    totalMinor: 4_200_000,
    currency: 'USD',
    reviewTier: 'high_value',
  }),
  'quote-unapproved-supplies': Object.freeze({
    quoteId: 'quote-unapproved-supplies',
    supplierId: 'supplier-quickbox-direct',
    supplierName: 'QuickBox Direct',
    supplierStatus: 'unapproved',
    category: 'office_supplies',
    itemName: 'Quarterly office supplies bundle',
    quantity: 10,
    unitPriceMinor: 9_000,
    totalMinor: 90_000,
    currency: 'USD',
    reviewTier: 'standard',
  }),
  'quote-restricted-gift-cards': Object.freeze({
    quoteId: 'quote-restricted-gift-cards',
    supplierId: 'supplier-northstar-office',
    supplierName: 'Northstar Office',
    supplierStatus: 'approved',
    category: 'gift_cards',
    itemName: 'Employee gift cards',
    quantity: 100,
    unitPriceMinor: 2_500,
    totalMinor: 250_000,
    currency: 'USD',
    reviewTier: 'standard',
  }),
};

export interface PublicProcurementQuote {
  quoteId: ProcurementQuoteId;
  supplierName: string;
  itemName: string;
  quantity: number;
  totalMinor: number;
  currency: 'USD';
}

export function procurementQuotes(): readonly PublicProcurementQuote[] {
  return PROCUREMENT_QUOTE_IDS.map((quoteId) => publicQuote(QUOTES[quoteId]));
}

export function findProcurementQuote(quoteId: ProcurementQuoteId): ProcurementQuote {
  return { ...QUOTES[quoteId] };
}

export function publicQuote(quote: ProcurementQuote): PublicProcurementQuote {
  return {
    quoteId: quote.quoteId,
    supplierName: quote.supplierName,
    itemName: quote.itemName,
    quantity: quote.quantity,
    totalMinor: quote.totalMinor,
    currency: quote.currency,
  };
}

export function normalizeProcurementPolicyIds(
  activePolicyIds: readonly ProcurementPolicyId[],
): ProcurementPolicyId[] {
  const selected = new Set(activePolicyIds);
  return PROCUREMENT_POLICY_IDS.filter((policyId) => selected.has(policyId));
}

export function procurementAgentId(activePolicyIds: readonly ProcurementPolicyId[]): string {
  const selected = new Set(normalizeProcurementPolicyIds(activePolicyIds));
  const bits = PROCUREMENT_POLICY_IDS.map((policyId) => (selected.has(policyId) ? '1' : '0'));
  return `procurement-demo-${bits.join('')}`;
}

interface ToolSelector {
  server_id: 'openai-agents';
  tool_name: 'submit_purchase_order';
}

interface ToolPolicyWhen {
  agents: string[];
  operations: ['submit_purchase_order'];
  side_effects: ['api_mutation'];
  tools: [ToolSelector];
}

interface ParameterMatcher {
  parameter: {
    path: string;
    equals?: string;
    one_of?: string[];
  };
}

export interface ProcurementToolPolicyDocument {
  family: 'tool';
  id: ProcurementPolicyId;
  description: string;
  severity: 'high' | 'critical';
  when: ToolPolicyWhen;
  match: ParameterMatcher;
  action: ProcurementPolicyEffect;
  reason: string;
  remediation: string;
  approver_roles?: ['owner', 'admin'];
  max_grant_ttl_seconds?: 900;
}

const POLICY_BITS: Readonly<Record<ProcurementPolicyId, number>> = {
  'procurement-approved-suppliers': 0b100,
  'procurement-high-value-review': 0b010,
  'procurement-restricted-categories': 0b001,
};

export function procurementPolicyDocuments(): readonly ProcurementToolPolicyDocument[] {
  return PROCUREMENT_POLICIES.map((policy) => policyDocument(policy));
}

export function procurementPolicyYaml(): readonly {
  id: ProcurementPolicyId;
  source: string;
}[] {
  return procurementPolicyDocuments().map((document) => ({
    id: document.id,
    source: stringify(document),
  }));
}

function policyDocument(policy: ProcurementPolicyDefinition): ProcurementToolPolicyDocument {
  const shared = {
    family: 'tool' as const,
    id: policy.id,
    description: policy.description,
    severity: policy.effect === 'require_approval' ? ('high' as const) : ('critical' as const),
    when: {
      agents: agentProfilesFor(policy.id),
      operations: ['submit_purchase_order'] as ['submit_purchase_order'],
      side_effects: ['api_mutation'] as ['api_mutation'],
      tools: [
        {
          server_id: 'openai-agents' as const,
          tool_name: 'submit_purchase_order' as const,
        },
      ] as [ToolSelector],
    },
  };

  if (policy.id === 'procurement-approved-suppliers') {
    return {
      ...shared,
      match: { parameter: { path: '/supplier_status', equals: 'unapproved' } },
      action: 'deny',
      reason: 'Purchase orders may only use approved suppliers.',
      remediation: 'Select a quote from an approved supplier.',
    };
  }
  if (policy.id === 'procurement-high-value-review') {
    return {
      ...shared,
      match: { parameter: { path: '/review_tier', equals: 'high_value' } },
      action: 'require_approval',
      reason: 'High-value purchase orders require human review.',
      remediation: 'Ask an owner or administrator to approve this exact purchase order.',
      approver_roles: ['owner', 'admin'],
      max_grant_ttl_seconds: 900,
    };
  }
  return {
    ...shared,
    match: {
      parameter: {
        path: '/category',
        one_of: ['gift_cards', 'weapons', 'personal_data'],
      },
    },
    action: 'deny',
    reason: 'This purchase category is restricted.',
    remediation: 'Remove restricted items from the request.',
  };
}

function agentProfilesFor(policyId: ProcurementPolicyId): string[] {
  const bit = POLICY_BITS[policyId];
  const profiles: string[] = [];
  for (let profile = 0; profile < 8; profile += 1) {
    if ((profile & bit) === bit)
      profiles.push(`procurement-demo-${profile.toString(2).padStart(3, '0')}`);
  }
  return profiles;
}
