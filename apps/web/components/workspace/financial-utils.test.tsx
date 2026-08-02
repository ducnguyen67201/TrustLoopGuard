import { describe, expect, it } from 'vitest';

import type { FinancialActionRecord } from '@featherlane-ai/sdk';

import {
  effectiveFinancialActionState,
  firstFailedFinancialEvidenceReason,
} from './financial-utils';

function action(overrides: Partial<FinancialActionRecord> = {}): FinancialActionRecord {
  return {
    id: 'action-1',
    workspace_id: 'workspace-1',
    environment_id: 'production',
    authorization_effect: 'defer',
    authorization_status: 'evaluating',
    execution_status: 'not_started',
    state: 'evaluating',
    action: {
      id: 'action-1',
      kind: 'refund',
      operation: 'issue_refund',
      principal_id: 'refund-bot',
      amount: { amount_minor: 12_500n, currency: 'USD' },
      rail: 'payment_http',
      metadata: { reason: 'item_arrived_damaged' },
    },
    evidence: [],
    created_at: '2026-07-15T19:41:57Z',
    updated_at: '2026-07-15T19:41:57Z',
    ...overrides,
  };
}

describe('financial action product state', () => {
  it('uses the Rust-projected state when present', () => {
    expect(effectiveFinancialActionState(action({ state: 'not_executable' }))).toBe(
      'not_executable',
    );
  });

  it('keeps a compatibility fallback for failed eligibility evidence', () => {
    const legacyAction = action({
        evidence: [
          {
            source: 'customer_backend',
            source_id: 'refund_eligibility_order-1',
            kind: 'refund_eligibility',
            metadata: {
              amount_lte_refundable_balance: false,
              refundable_balance_minor: 10_000,
            },
          },
        ],
      });
    delete (legacyAction as Partial<FinancialActionRecord>).state;

    expect(effectiveFinancialActionState(legacyAction)).toBe('not_executable');
    expect(firstFailedFinancialEvidenceReason(legacyAction)).toBe(
      'Amount exceeds refundable balance',
    );
  });

  it('maps evaluated denials and successful execution without guessing from outcomes', () => {
    expect(
      effectiveFinancialActionState(
        action({
          authorization_intent_id: 'intent-1',
          authorization_effect: 'deny',
          authorization_status: 'denied',
          state: 'blocked',
        }),
      ),
    ).toBe('blocked');
    expect(
      effectiveFinancialActionState(
        action({
          authorization_effect: 'permit',
          authorization_status: 'authorized',
          execution_status: 'succeeded',
          state: 'executed',
        }),
      ),
    ).toBe('executed');
  });
});
