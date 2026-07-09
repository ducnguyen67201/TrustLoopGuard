import { describe, expect, it } from 'vitest';
import {
  financialExecutionAttestationMessage,
  signFinancialExecutionAttestation,
} from '../src/financial-attestation';

const request = {
  connector_id: 'connector-1',
  grant_id: 'grant-1',
  action_hash: 'sha256:action',
  provider: 'stripe',
  provider_reference: 'pi_123',
  provider_status: 'succeeded',
  executed_at: '2026-07-09T00:00:00Z',
  idempotency_key: 'commit-1',
  provider_proof: 'provider receipt',
  provider_proof_sha256: 'sha256:proof',
};

describe('financial execution attestation', () => {
  it('matches the shared HMAC-SHA256 vector', async () => {
    const message = new TextDecoder().decode(
      financialExecutionAttestationMessage('action-1', request),
    );
    expect(message).toContain('\n11:connector-1\n8:action-1\n7:grant-1');

    await expect(
      signFinancialExecutionAttestation(
        'MDEyMzQ1Njc4OTAxMjM0NTY3ODkwMTIzNDU2Nzg5MDE',
        'action-1',
        request,
      ),
    ).resolves.toBe('v1=FbjzlmAsFdGVBB5yKbLD6UZ6-CgtIXZCtByHv49nXpY');
  });

  it('binds every field into the signature', async () => {
    const secret = 'MDEyMzQ1Njc4OTAxMjM0NTY3ODkwMTIzNDU2Nzg5MDE';
    const original = await signFinancialExecutionAttestation(secret, 'action-1', request);
    const changed = await signFinancialExecutionAttestation(secret, 'action-1', {
      ...request,
      provider_reference: 'pi_changed',
    });
    expect(changed).not.toBe(original);
  });
});
