import { describe, expect, it } from 'vitest';

import { redactCheckRequest, type CheckRequest } from '../src';

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
});
