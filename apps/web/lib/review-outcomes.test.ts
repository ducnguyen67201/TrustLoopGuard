import { buildReviewEventPayload, canSubmitReviewOutcome } from './review-outcomes';

const payload = buildReviewEventPayload({
  outcome: 'corrected',
  reasonCodes: ['field_mismatch', 'bad_input_quality'],
  note: '  corrected the extracted T4 amount  ',
});

if (payload.outcome !== 'corrected') {
  throw new Error('expected selected outcome');
}

if (payload.reason_codes.join(',') !== 'field_mismatch,bad_input_quality') {
  throw new Error('expected selected reason codes');
}

if (payload.note !== 'corrected the extracted T4 amount') {
  throw new Error('expected note to be trimmed');
}

if (payload.metadata.source !== 'dashboard') {
  throw new Error('expected dashboard metadata source');
}

const payloadWithoutNote = buildReviewEventPayload({
  outcome: 'accepted',
  reasonCodes: [],
  note: '   ',
});

if ('note' in payloadWithoutNote) {
  throw new Error('expected blank note to be omitted');
}

if (!canSubmitReviewOutcome('accepted')) {
  throw new Error('expected accepted outcome to be submittable');
}

if (canSubmitReviewOutcome('')) {
  throw new Error('expected blank outcome to be rejected');
}
