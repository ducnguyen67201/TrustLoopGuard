import assert from 'node:assert/strict';
import test from 'node:test';

import type {
  AuthorizationApproval,
  AuthorizationReceipt,
  Client,
  FinancialActionRecord,
  FinancialReceipt,
} from '@trustloopguard/sdk';
import { readRefundDemoActionStatus, type RefundDemoStatusClient } from './status';

type ExecuteFinancialActionRequest = NonNullable<Parameters<Client['executeAction']>[1]>;

test('reads linked authorization and execution state for a completed refund', async () => {
  const actionId = '019f5d63-f8ca-77c3-ae7f-07b122daa7b3';
  const client: RefundDemoStatusClient = {
    async getFinancialAction() { return demoAction(actionId); },
    async getAuthorizationReceipt() { throw new Error('not expected'); },
    async getApproval() { throw new Error('not expected'); },
    async executeAction() { throw new Error('not expected'); },
    async getReceipt(): Promise<FinancialReceipt> {
      return {
        id: actionId, action_id: actionId, authorization_receipt_id: `authorization_${actionId}`,
        ledger_event_ids: [], proof: { provider: { reference: 're_test_approved_123', status: 'succeeded' } },
        created_at: '2026-07-13T21:31:00.000Z',
      };
    },
  };

  assert.deepEqual(await readRefundDemoActionStatus(client, actionId), {
    actionId, authorizationEffect: 'permit', executionStatus: 'succeeded',
    orderId: 'ord_demo_1001', amountMinor: 7_500, currency: 'USD', receiptId: actionId,
    providerReference: 're_test_approved_123', updatedAt: '2026-07-13T21:31:00.000Z',
  });
});

test('refuses to expose a non-demo financial action', async () => {
  const action = demoAction('019f5d63-f8ca-77c3-ae7f-07b122daa7b3');
  action.action.principal_id = 'another-agent';
  const client: RefundDemoStatusClient = {
    async getFinancialAction() { return action; },
    async getAuthorizationReceipt() { throw new Error('not expected'); },
    async getApproval() { throw new Error('not expected'); },
    async executeAction() { throw new Error('not expected'); },
    async getReceipt() { throw new Error('not expected'); },
  };
  await assert.rejects(readRefundDemoActionStatus(client, action.id), /not found/i);
});

test('executes an approved held refund through its approval grant', async () => {
  const actionId = '019f5d63-f8ca-77c3-ae7f-07b122daa7b3';
  const approvalId = `approval_${actionId}`;
  const grantId = `grant_${actionId}`;
  const held = demoAction(actionId);
  held.authorization_effect = 'require_approval';
  held.authorization_status = 'pending_approval';
  held.execution_status = 'not_started';
  held.state = 'held_for_approval';
  const executed = {
    ...held,
    authorization_effect: 'permit' as const,
    authorization_status: 'authorized' as const,
    execution_status: 'succeeded' as const,
    state: 'executed' as const,
    updated_at: '2026-07-13T21:32:00.000Z',
  };
  let executeRequest: ExecuteFinancialActionRequest | undefined;
  const client: RefundDemoStatusClient = {
    async getFinancialAction() { return held; },
    async getAuthorizationReceipt(): Promise<AuthorizationReceipt> {
      return authorizationReceipt(held.authorization_receipt_id!, approvalId);
    },
    async getApproval(): Promise<AuthorizationApproval> {
      return approvalForAction(held, approvalId, grantId);
    },
    async executeAction(receivedActionId, request) {
      assert.equal(receivedActionId, actionId);
      executeRequest = request;
      return executed;
    },
    async getReceipt(): Promise<FinancialReceipt> {
      return {
        id: actionId,
        action_id: actionId,
        authorization_receipt_id: executed.authorization_receipt_id!,
        ledger_event_ids: [`${actionId}:executed`],
        proof: { provider: { reference: 're_test_approved_123', status: 'succeeded' } },
        created_at: executed.updated_at,
      };
    },
  };

  assert.deepEqual(await readRefundDemoActionStatus(client, actionId), {
    actionId,
    authorizationEffect: 'permit',
    executionStatus: 'succeeded',
    orderId: 'ord_demo_1001',
    amountMinor: 7_500,
    currency: 'USD',
    receiptId: actionId,
    providerReference: 're_test_approved_123',
    updatedAt: '2026-07-13T21:32:00.000Z',
  });
  assert.deepEqual(executeRequest, {
    authorization: {
      grant_id: grantId,
      attempt_id: `stripe-refund-agent:execute:${actionId}`,
    },
    attempt_id: `stripe-refund-agent:execute:${actionId}`,
  });
});

function demoAction(id: string): FinancialActionRecord {
  return {
    id, workspace_id: 'default', environment_id: 'production',
    authorization_intent_id: `intent_${id}`, authorization_receipt_id: `authorization_${id}`,
    authorization_effect: 'permit', authorization_status: 'authorized', execution_status: 'succeeded',
    state: 'executed',
    action: {
      id, kind: 'refund', operation: 'issue_refund', principal_id: 'refund-bot',
      amount: { amount_minor: 7_500n, currency: 'USD' },
      counterparty: { id: 'cust_demo_1001', kind: 'customer', metadata: null },
      rail: 'payment_http', memo: 'Refund ord_demo_1001: item_arrived_damaged',
      metadata: { demo_request_id: 'demo-request-123', order_id: 'ord_demo_1001', reason: 'item_arrived_damaged' },
    },
    evidence: [], created_at: '2026-07-13T21:30:00.000Z', updated_at: '2026-07-13T21:31:00.000Z',
  };
}

function authorizationReceipt(id: string, approvalId: string): AuthorizationReceipt {
  return {
    id,
    intent_id: `intent_${id}`,
    domain: 'financial',
    effect: 'require_approval',
    intent_status: 'pending_approval',
    subject_hash: 'sha256:v1:test',
    reason: 'approval required',
    findings: [],
    policy_versions: ['refund-bot-refund-controls'],
    approval_id: approvalId,
    domain_evidence: { domain: 'financial', evidence: { action_id: id } },
    created_at: '2026-07-13T21:31:00.000Z',
  };
}

function approvalForAction(
  action: FinancialActionRecord,
  approvalId: string,
  grantId: string,
): AuthorizationApproval {
  return {
    id: approvalId,
    workspace_id: action.workspace_id,
    environment_id: action.environment_id,
    intent_id: action.authorization_intent_id!,
    status: 'approved',
    envelope: {
      schema: 'authorization-envelope:v1',
      intent_id: action.authorization_intent_id!,
      domain: 'financial',
      capability: 'financial:issue_refund',
      principal_id: 'refund-bot',
      subject_id: action.id,
      subject_hash: 'sha256:v1:test',
      exact_fingerprint: 'sha256:v1:test',
      fingerprint_version: 1,
      requirement_ids: ['financial:refund-bot-refund-controls:approval_threshold'],
      policy_versions: ['refund-bot-refund-controls'],
      issued_at: '2026-07-13T21:31:00.000Z',
      expires_at: '2026-07-13T21:46:00.000Z',
    },
    envelope_hash: 'sha256:v1:envelope',
    approver_roles: [],
    decided_by: 'user_1',
    decided_at: '2026-07-13T21:31:10.000Z',
    grant_id: grantId,
    expires_at: '2026-07-13T21:46:00.000Z',
    created_at: '2026-07-13T21:31:00.000Z',
    updated_at: '2026-07-13T21:31:10.000Z',
  };
}
