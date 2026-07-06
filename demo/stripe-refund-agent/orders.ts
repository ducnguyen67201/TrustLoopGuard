import {
  DEMO_CUSTOMER_ID,
  DEMO_ORDER_ID,
  DEMO_PAYMENT_METHOD_ID,
  type OrderRecord,
  type OrderSearchQuery,
  type OrderSearchResult,
  type RefundEvidence,
} from './types';

export function demoOrders(): OrderRecord[] {
  return [
    {
      id: DEMO_ORDER_ID,
      customerId: DEMO_CUSTOMER_ID,
      customerEmail: 'jamie@example.com',
      customerName: 'Jamie Demo',
      paymentIntentId: process.env.STRIPE_PAYMENT_INTENT_ID?.trim() || 'pi_demo_seeded_refund',
      paymentMethodId: DEMO_PAYMENT_METHOD_ID,
      amountPaidMinor: 10_000,
      refundableBalanceMinor: 10_000,
      currency: 'USD',
      captured: true,
      refundWindowOpen: true,
      refundCount: 0,
    },
  ];
}

export function searchOrder(query: OrderSearchQuery, orders = demoOrders()): OrderSearchResult {
  const normalizedOrderId = query.orderId?.trim().toLowerCase();
  const normalizedEmail = query.email?.trim().toLowerCase();
  const normalizedLast4 = query.last4?.trim();

  const order = orders.find((candidate) => {
    if (normalizedOrderId && candidate.id.toLowerCase() === normalizedOrderId) return true;
    if (normalizedEmail && candidate.customerEmail.toLowerCase() === normalizedEmail) return true;
    return normalizedLast4 === '4242' && candidate.paymentMethodId === DEMO_PAYMENT_METHOD_ID;
  });

  if (order === undefined) {
    return {
      found: false,
      evidence: emptyEvidence(),
      evidenceRef: 'refund_eligibility_missing_order',
      reason: 'order not found',
    };
  }

  return {
    found: true,
    order,
    evidence: evidenceForOrder(order),
    evidenceRef: `refund_eligibility_${order.id}`,
  };
}

export function evidenceForOrder(order: OrderRecord): RefundEvidence {
  return {
    orderExists: true,
    paymentCaptured: order.captured,
    refundWindowOpen: order.refundWindowOpen,
    amountLteRefundableBalance: true,
    destinationIsOriginalPaymentMethod: true,
    noDuplicateRefund: order.refundCount === 0,
  };
}

function emptyEvidence(): RefundEvidence {
  return {
    orderExists: false,
    paymentCaptured: false,
    refundWindowOpen: false,
    amountLteRefundableBalance: false,
    destinationIsOriginalPaymentMethod: false,
    noDuplicateRefund: false,
  };
}
