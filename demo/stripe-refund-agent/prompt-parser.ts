import { DEMO_ORDER_ID, type PrepareRefundInput } from './types';

export function refundInputFromPrompt(prompt: string): PrepareRefundInput {
  return {
    orderId: orderIdFromPrompt(prompt),
    amountMinor: amountMinorFromPrompt(prompt),
    reason: reasonFromPrompt(prompt),
  };
}

function orderIdFromPrompt(prompt: string): string {
  return prompt.match(/ord_[a-z0-9_]+/i)?.[0] ?? DEMO_ORDER_ID;
}

function amountMinorFromPrompt(prompt: string): number {
  const dollars =
    prompt.match(/\$\s*(\d+(?:\.\d{1,2})?)/)?.[1] ??
    prompt.match(/\bfor\s+(\d+(?:\.\d{1,2})?)\b/i)?.[1];
  if (dollars === undefined) return 7_500;
  return Math.round(Number.parseFloat(dollars) * 100);
}

function reasonFromPrompt(prompt: string): string {
  return prompt.toLowerCase().includes('damaged') ? 'damaged_item' : 'customer_request';
}
