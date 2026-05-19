export type ReviewOutcome =
  | 'accepted'
  | 'corrected'
  | 'rejected'
  | 'false_positive'
  | 'missed_issue'
  | 'ignored';

export type ReviewReasonCode =
  | 'field_mismatch'
  | 'bad_input_quality'
  | 'missing_document'
  | 'policy_noise'
  | 'pii_risk'
  | 'requires_accountant_judgment'
  | 'unsupported_claim';

export type ReviewOutcomeOption = {
  value: ReviewOutcome;
  label: string;
};

export type ReviewReasonOption = {
  value: ReviewReasonCode;
  label: string;
};

export type BuildReviewEventPayloadInput = {
  outcome: ReviewOutcome;
  reasonCodes: ReviewReasonCode[];
  note: string;
};

export type ReviewEventPayload = {
  outcome: ReviewOutcome;
  reason_codes: ReviewReasonCode[];
  note?: string;
  metadata: {
    source: 'dashboard';
  };
};

export const REVIEW_OUTCOME_OPTIONS: ReviewOutcomeOption[] = [
  { value: 'accepted', label: 'Accepted' },
  { value: 'corrected', label: 'Corrected' },
  { value: 'rejected', label: 'Rejected' },
  { value: 'false_positive', label: 'False positive' },
  { value: 'missed_issue', label: 'Missed issue' },
  { value: 'ignored', label: 'Ignored' },
];

export const REVIEW_REASON_OPTIONS: ReviewReasonOption[] = [
  { value: 'field_mismatch', label: 'Field mismatch' },
  { value: 'bad_input_quality', label: 'Bad input quality' },
  { value: 'missing_document', label: 'Missing document' },
  { value: 'policy_noise', label: 'Policy noise' },
  { value: 'pii_risk', label: 'PII risk' },
  { value: 'requires_accountant_judgment', label: 'Requires accountant judgment' },
  { value: 'unsupported_claim', label: 'Unsupported claim' },
];

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
