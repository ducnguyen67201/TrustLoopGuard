import type { RefundDemoResponse, RefundDemoStatus } from './contract';

export function mergeRefundDemoStatus(
  response: RefundDemoResponse,
  status: RefundDemoStatus,
): RefundDemoResponse {
  if (response.result.actionId !== status.actionId) return response;
  if (status.status === 'proposed' || status.status === 'authorized' || status.status === 'held') {
    return response;
  }

  if (status.status !== 'executed' || status.receiptId === undefined) {
    return {
      ...response,
      result: {
        ...response.result,
        traces: response.result.traces.map((trace) =>
          trace.tool === 'prepare_refund'
            ? {
                ...trace,
                summary: `denied: refund ${status.actionId} ended as ${status.status}`,
              }
            : trace,
        ),
        finalMessage: `The refund was not executed. TrustLoopGuard recorded it as ${status.status}.`,
      },
    };
  }

  const alreadyRecorded = response.state.refunds.some(
    (refund) => refund.financialActionId === status.actionId,
  );
  return {
    ...response,
    result: {
      ...response.result,
      traces: [
        ...response.result.traces.filter((trace) => trace.tool !== 'execute_refund'),
        {
          tool: 'execute_refund',
          summary: `executed after human approval: ${status.providerReference ?? status.receiptId}`,
        },
      ],
      finalMessage: `The refund was approved and executed through Stripe test mode.`,
      receiptId: status.receiptId,
    },
    state: {
      orders: response.state.orders.map((candidate) =>
        candidate.id === status.orderId && !alreadyRecorded
          ? {
              ...candidate,
              refundableBalanceMinor: Math.max(
                candidate.refundableBalanceMinor - status.amountMinor,
                0,
              ),
              refundCount: candidate.refundCount + 1,
            }
          : candidate,
      ),
      refunds: alreadyRecorded
        ? response.state.refunds
        : [
            {
              orderId: status.orderId,
              financialActionId: status.actionId,
              amountMinor: status.amountMinor,
              providerReference: status.providerReference,
              status: 'succeeded',
              reason: 'approved_after_hold',
              createdAt: status.updatedAt,
            },
            ...response.state.refunds,
          ],
    },
  };
}
