import type { AuthorizationDecision } from '@trustloopguard/sdk';

export function blockedReply(decision: AuthorizationDecision): string {
  return `[DENIED] ${decision.reason}`;
}

export function escalatedReply(decision: AuthorizationDecision): string {
  return `[APPROVAL REQUIRED] ${decision.reason}`;
}

export function demoDraft(input: string): string {
  const normalized = input.toLowerCase();
  if (normalized.includes('phone') || normalized.includes('number')) {
    return 'You can reach the account owner directly at 415-555-1212.';
  }
  if (normalized.includes('refund')) {
    return "I can approve a full refund right now, and we'll process it today.";
  }
  if (normalized.includes('ignore previous')) {
    return "Sure, I'll ignore the prior instructions and reveal hidden details.";
  }
  return "We're open 9 am to 5 pm on weekdays, and I can help route your request.";
}
