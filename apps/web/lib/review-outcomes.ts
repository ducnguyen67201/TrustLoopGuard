type ReviewOutcome =
  | 'accepted'
  | 'corrected'
  | 'rejected'
  | 'false_positive'
  | 'missed_issue'
  | 'ignored';

type ReviewReasonCode =
  | 'field_mismatch'
  | 'bad_input_quality'
  | 'missing_document'
  | 'policy_noise'
  | 'pii_risk'
  | 'requires_accountant_judgment'
  | 'unsupported_claim';

type BuildReviewEventPayloadInput = {
  outcome: ReviewOutcome;
  reasonCodes: ReviewReasonCode[];
  note: string;
};

type ReviewEventPayload = {
  outcome: ReviewOutcome;
  reason_codes: ReviewReasonCode[];
  note?: string;
  metadata: {
    source: 'dashboard';
  };
};

export function canSubmitReviewOutcome(outcome: ReviewOutcome | ''): outcome is ReviewOutcome {
  return outcome !== '';
}

export function buildReviewEventPayload({
  outcome,
  reasonCodes,
  note,
}: BuildReviewEventPayloadInput): ReviewEventPayload {
  const trimmedNote = note.trim();
  const payload: ReviewEventPayload = {
    outcome,
    reason_codes: reasonCodes,
    metadata: { source: 'dashboard' },
  };

  if (trimmedNote !== '') {
    return { ...payload, note: trimmedNote };
  }

  return payload;
}
