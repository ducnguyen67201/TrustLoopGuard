import { buildReviewEventPayload, canSubmitReviewOutcome } from './review-outcomes';
import { describe, expect, it } from 'vitest';

describe('review outcome payloads', () => {
  it('builds a typed dashboard review event payload', () => {
    const payload = buildReviewEventPayload({
      outcome: 'corrected',
      reasonCodes: ['field_mismatch', 'bad_input_quality'],
      note: '  corrected the extracted T4 amount  ',
    });

    expect(payload.outcome).toBe('corrected');
    expect(payload.reason_codes).toEqual(['field_mismatch', 'bad_input_quality']);
    expect(payload.note).toBe('corrected the extracted T4 amount');
    expect(payload.metadata.source).toBe('dashboard');
  });

  it('omits blank notes', () => {
    const payloadWithoutNote = buildReviewEventPayload({
      outcome: 'accepted',
      reasonCodes: [],
      note: '   ',
    });

    expect(payloadWithoutNote).not.toHaveProperty('note');
  });

  it('allows submission only after an outcome is selected', () => {
    expect(canSubmitReviewOutcome('accepted')).toBe(true);
    expect(canSubmitReviewOutcome('')).toBe(false);
  });
});
