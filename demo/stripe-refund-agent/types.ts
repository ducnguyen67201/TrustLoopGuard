import type {
  AuthorizationEffect,
  CreateFinancialActionRequest,
  FinancialExecutionStatus,
  FinancialActionRecord,
  FinancialReceipt,
} from '@trustloopguard/sdk';

export const REFUND_AGENT_ID = 'refund-bot';
export const REFUND_GRANT_CAPABILITY = 'financial:issue_refund';
export const REFUND_GRANT_REQUIREMENT_IDS = [
  'financial:refund-bot-refund-controls:grant_required',
  'financial:refund-bot-refund-controls:approval_threshold',
];
export const DEMO_ORDER_ID = 'ord_demo_1001';
export const DEMO_CUSTOMER_ID = 'cust_demo_1001';
export const DEMO_PAYMENT_METHOD_ID = 'pm_card_visa';
export const DEFAULT_PROVIDER_PORT = 9303;
export const DEFAULT_PROVIDER_API_KEY = 'stripe-refund-demo-token';

export interface OrderRecord {
  id: string;
  customerId: string;
  customerEmail: string;
  customerName: string;
  paymentIntentId: string;
  paymentMethodId: string;
  paymentMethodLast4: string;
  amountPaidMinor: number;
  refundableBalanceMinor: number;
  currency: 'USD';
  captured: boolean;
  refundWindowOpen: boolean;
  refundCount: number;
}

export interface RefundRecord {
  id: number;
  orderId: string;
  financialActionId: string;
  amountMinor: number;
  providerReference?: string;
  status: string;
  reason: string;
  createdAt: string;
}

export interface CustomerBackendState {
  orders: OrderRecord[];
  refunds: RefundRecord[];
}

export interface OrderSearchQuery {
  orderId?: string;
  email?: string;
  last4?: string;
}

export interface RefundEvidence {
  orderExists: boolean;
  paymentCaptured: boolean;
  refundWindowOpen: boolean;
  amountLteRefundableBalance: boolean;
  destinationIsOriginalPaymentMethod: boolean;
  noDuplicateRefund: boolean;
}

export interface OrderSearchResult {
  found: boolean;
  order?: OrderRecord;
  evidence: RefundEvidence;
  evidenceRef: string;
  reason?: string;
}

export interface PrepareRefundInput {
  orderId: string;
  amountMinor: number;
  reason: string;
  requestId?: string;
}

export interface PrepareRefundResult {
  action: FinancialActionRecord;
  request: CreateFinancialActionRequest;
  order: OrderRecord;
  status: AuthorizationEffect;
  message: string;
}

export interface ExecuteRefundResult {
  action: FinancialActionRecord;
  receipt?: FinancialReceipt;
  status: FinancialExecutionStatus | AuthorizationEffect;
  message: string;
}

export interface ToolTrace {
  tool: 'search_order' | 'prepare_refund' | 'execute_refund';
  summary: string;
}

export interface AgentRunResult {
  prompt: string;
  traces: ToolTrace[];
  finalMessage: string;
  actionId?: string;
  receiptId?: string;
}

export interface AgentRunLogger {
  log(step: string, message: string): void;
}

export interface AgentRunLogEntry {
  step: string;
  message: string;
}

export interface AgentRunOptions {
  useOpenAI?: boolean;
  requireLiveAgent?: boolean;
  logger?: AgentRunLogger;
  requestId?: string;
  dbPath?: string;
  refundGrantId?: string;
  allowGrantProvisioning?: boolean;
}

export interface StripeRefundProviderRequest {
  action_id: string;
  kind: string;
  amount?: number;
  amount_minor?: number;
  currency: string;
  memo?: string;
  metadata?: {
    payment_intent_id?: string;
    order_id?: string;
    reason?: string;
  };
}

export interface StripeRefundProviderResponse {
  status: 'succeeded' | 'failed';
  provider_status: string;
  provider_reference: string;
  reversal_capability: 'manual_recovery' | 'none';
  recovery_status: 'manual_required' | 'not_needed';
  mode: 'simulated' | 'stripe-test';
  stripe_refund_id?: string;
}

export interface StripeRefundResult {
  id: string;
  status: string;
}
