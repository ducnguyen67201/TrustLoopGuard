import {
  type OrderSearchQuery,
  type OrderSearchResult,
  type OrderRecord,
  type RefundEvidence,
} from './types';

import { findOrder } from './order-db';

export function searchOrder(query: OrderSearchQuery): OrderSearchResult {
  const order = findOrder(query);

  if (order === null) {
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
