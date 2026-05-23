import { describe, expect, it } from 'vitest';

import { redactCheckRequest, type CheckRequest, type RedactionEntityType } from '../src';

function baseRequest(overrides: Partial<CheckRequest> = {}): CheckRequest {
  return {
    agent_id: 'tax-document-agent',
    channel: 'chat',
    input: '',
    proposed_output: '',
    domain: null,
    policies: [],
    context: {},
    trace_id: null,
    ...overrides,
  };
}

describe('redactCheckRequest', () => {
  it('builds a sanitized request without mutating the caller object', () => {
    const original: CheckRequest = {
      agent_id: 'tax-document-agent',
      channel: 'chat',
      input: 'Alice Example has SIN 123-456-789.',
      proposed_output: 'Email alice@example.com about income $82,000.',
      domain: null,
      policies: [],
      context: {
        workflow_step: 'document_extraction',
        notes: 'Alice Example uploaded T4 slip T4-2025-0001.',
      },
      trace_id: null,
    };

    const result = redactCheckRequest(original, {
      mode: 'sdk_local',
      entities: ['PERSON_NAME', 'SIN', 'EMAIL', 'INCOME_AMOUNT', 'TAX_FORM_ID'],
    });

    expect(original.input).toBe('Alice Example has SIN 123-456-789.');
    expect(original.proposed_output).toBe('Email alice@example.com about income $82,000.');
    expect(original.context).toEqual({
      workflow_step: 'document_extraction',
      notes: 'Alice Example uploaded T4 slip T4-2025-0001.',
    });

    const outbound = JSON.stringify(result.request);
    expect(outbound).not.toContain('Alice Example');
    expect(outbound).not.toContain('123-456-789');
    expect(outbound).not.toContain('alice@example.com');
    expect(outbound).not.toContain('$82,000');
    expect(outbound).toContain('[PERSON_NAME_1]');
    expect(outbound).toContain('[SIN_1]');
    expect(outbound).toContain('[EMAIL_1]');
    expect(outbound).toContain('[INCOME_AMOUNT_1]');
    expect(result.request.redaction.status).toBe('applied');
    expect(result.tokenMap.get('[EMAIL_1]')).toBe('alice@example.com');
  });

  it('applies a fixed precedence regardless of caller-supplied order', () => {
    // PERSON_NAME's any-two-capitalized-words pattern could swallow
    // narrower matches if it ran first. The helper must apply
    // most-specific patterns first regardless of caller order.
    const base: CheckRequest = {
      agent_id: 'tax-document-agent',
      channel: 'chat',
      input: 'Alice Example has SIN 123-456-789.',
      proposed_output: '',
      domain: null,
      policies: [],
      context: {},
      trace_id: null,
    };

    const personFirst = redactCheckRequest(base, {
      mode: 'sdk_local',
      entities: ['PERSON_NAME', 'SIN'],
    });
    const sinFirst = redactCheckRequest(base, {
      mode: 'sdk_local',
      entities: ['SIN', 'PERSON_NAME'],
    });

    expect(personFirst.request.input).toBe(sinFirst.request.input);
    expect(personFirst.request.input).toContain('[SIN_1]');
    expect(personFirst.request.input).toContain('[PERSON_NAME_1]');
  });

  it('returns applied status with no redactions when entities list is empty', () => {
    const req = baseRequest({
      input: 'Contact alice@example.com.',
      proposed_output: 'No PII to remove.',
    });

    const { request, tokenMap } = redactCheckRequest(req, {
      mode: 'sdk_local',
      entities: [],
    });

    expect(request.input).toBe('Contact alice@example.com.');
    expect(request.proposed_output).toBe('No PII to remove.');
    expect(request.redaction.status).toBe('applied');
    expect(request.redaction.input_redacted).toBe(false);
    expect(request.redaction.proposed_output_redacted).toBe(false);
    expect(request.redaction.context_redacted).toBe(false);
    expect(request.redaction.entities).toEqual([]);
    expect(tokenMap.size).toBe(0);
  });

  it('reuses one token and accumulates the count for repeated raw values', () => {
    const req = baseRequest({
      input: 'Reach alice@example.com or alice@example.com.',
      proposed_output: 'Confirm with alice@example.com.',
    });

    const { request } = redactCheckRequest(req, {
      mode: 'sdk_local',
      entities: ['EMAIL'],
    });

    expect(request.input).not.toContain('alice@example.com');
    expect(request.proposed_output).not.toContain('alice@example.com');
    expect(request.input.match(/\[EMAIL_1\]/g)?.length).toBe(2);

    const emailEntity = request.redaction.entities.find(
      (entity) => entity.entity_type === 'EMAIL',
    );
    expect(emailEntity).toBeDefined();
    expect(emailEntity!.token).toBe('[EMAIL_1]');
    expect(emailEntity!.count).toBe(3);
  });

  it('preserves passthrough context keys and only redacts the rest', () => {
    const req = baseRequest({
      context: {
        workflow_step: 'alice@example.com',
        document_type: 'alice@example.com',
        confidence_bucket: 'alice@example.com',
        pii_types: ['alice@example.com'],
        notes: 'alice@example.com',
      },
    });

    const { request } = redactCheckRequest(req, {
      mode: 'sdk_local',
      entities: ['EMAIL'],
    });

    const context = request.context as Record<string, unknown>;
    expect(context.workflow_step).toBe('alice@example.com');
    expect(context.document_type).toBe('alice@example.com');
    expect(context.confidence_bucket).toBe('alice@example.com');
    expect(context.pii_types).toEqual(['alice@example.com']);
    expect(context.notes).toBe('[EMAIL_1]');
    expect(request.redaction.context_redacted).toBe(true);
  });

  it('does not redact bare numeric runs when BANK_ACCOUNT lacks an account keyword', () => {
    // Regression: BANK_ACCOUNT previously matched any 7-17 digit sequence,
    // destroying order IDs and other benign numeric identifiers.
    const req = baseRequest({
      input: 'Order 12345678 shipped to confirmation 9876543210.',
      proposed_output: 'Reference 1122334455.',
    });

    const { request } = redactCheckRequest(req, {
      mode: 'sdk_local',
      entities: ['BANK_ACCOUNT'] as RedactionEntityType[],
    });

    expect(request.input).toBe('Order 12345678 shipped to confirmation 9876543210.');
    expect(request.proposed_output).toBe('Reference 1122334455.');
    expect(request.redaction.entities).toEqual([]);
  });

  it('redacts bank account numbers when prefixed with an account keyword', () => {
    const req = baseRequest({
      input: 'Account number 12345678901 on file.',
      proposed_output: 'A/C: 9876543210 verified.',
    });

    const { request } = redactCheckRequest(req, {
      mode: 'sdk_local',
      entities: ['BANK_ACCOUNT'] as RedactionEntityType[],
    });

    expect(request.input).not.toContain('12345678901');
    expect(request.proposed_output).not.toContain('9876543210');
    expect(request.input).toContain('[BANK_ACCOUNT_1]');
    expect(request.proposed_output).toContain('[BANK_ACCOUNT_2]');
  });
});
