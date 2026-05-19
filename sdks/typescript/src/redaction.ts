import type { CheckRequest } from './generated/CheckRequest';
import type { RedactedEntity } from './generated/RedactedEntity';
import type { RedactionInfo } from './generated/RedactionInfo';
import type { RedactionMode } from './generated/RedactionMode';
import type { RedactionStatus } from './generated/RedactionStatus';

export type { RedactedEntity, RedactionInfo, RedactionMode, RedactionStatus };

export type RedactionEntityType =
  | 'PERSON_NAME'
  | 'SIN'
  | 'DATE_OF_BIRTH'
  | 'ADDRESS'
  | 'PHONE_NUMBER'
  | 'EMAIL'
  | 'INCOME_AMOUNT'
  | 'EMPLOYER_NAME'
  | 'TAX_FORM_ID'
  | 'BANK_ACCOUNT'
  | 'GOVERNMENT_ID'
  | 'CUSTOM';

export interface RedactionOptions {
  mode: Extract<RedactionMode, 'sdk_local'>;
  entities: RedactionEntityType[];
}

export type RedactedCheckRequest = CheckRequest & { redaction: RedactionInfo };

export interface RedactionResult {
  request: RedactedCheckRequest;
  tokenMap: Map<string, string>;
}

type JsonLike = null | boolean | number | string | JsonLike[] | { [key: string]: JsonLike };

const CONTEXT_PASSTHROUGH_KEYS = new Set([
  'workflow_step',
  'document_type',
  'confidence_bucket',
  'pii_types',
]);

const PATTERNS: Record<RedactionEntityType, RegExp> = {
  EMAIL: /\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}\b/g,
  SIN: /\b\d{3}[- ]?\d{3}[- ]?\d{3}\b/g,
  INCOME_AMOUNT: /\$\d{1,3}(?:,\d{3})*(?:\.\d{2})?\b/g,
  TAX_FORM_ID: /\bT\d-[0-9A-Za-z-]+\b/g,
  PERSON_NAME: /\b[A-Z][a-z]+ [A-Z][a-z]+\b/g,
  DATE_OF_BIRTH: /\b(?:19|20)\d{2}-\d{2}-\d{2}\b/g,
  PHONE_NUMBER: /\b(?:\+?1[-. ]?)?\(?\d{3}\)?[-. ]?\d{3}[-. ]?\d{4}\b/g,
  ADDRESS: /\b\d{1,6} [A-Z][A-Za-z]*(?: [A-Z][A-Za-z]*)* (?:Street|St|Avenue|Ave|Road|Rd)\b/g,
  EMPLOYER_NAME: /\b[A-Z][A-Za-z]+ (?:Inc|LLC|Ltd|Corp|Corporation)\b/g,
  // Requires an account-context keyword; matching bare 7–17 digit runs would
  // destroy benign numeric identifiers (order IDs, codes, timestamps).
  BANK_ACCOUNT: /(?<=\b(?:account|acct|a\/c)(?:\s*(?:no\.?|number|#))?[\s:]*)\d{7,17}\b/gi,
  GOVERNMENT_ID: /\b[A-Z]{2}\d{6,10}\b/g,
  CUSTOM: /$a/g,
};

// Application order is fixed regardless of caller order: most-specific
// first so loose patterns (PERSON_NAME's any-two-capitalized-words,
// EMPLOYER_NAME's suffix-based match) cannot swallow narrower matches
// like SIN or PHONE_NUMBER. Caller-supplied `entities` filters which
// patterns run; it does not control precedence.
const APPLICATION_ORDER: readonly RedactionEntityType[] = [
  'TAX_FORM_ID',
  'EMAIL',
  'SIN',
  'DATE_OF_BIRTH',
  'PHONE_NUMBER',
  'GOVERNMENT_ID',
  'BANK_ACCOUNT',
  'INCOME_AMOUNT',
  'ADDRESS',
  'EMPLOYER_NAME',
  'PERSON_NAME',
  'CUSTOM',
];

export function redactCheckRequest(
  req: CheckRequest,
  options: RedactionOptions,
): RedactionResult {
  const redactor = new LocalRedactor(options.entities);
  const input = redactor.redactText(req.input);
  const proposedOutput = redactor.redactText(req.proposed_output);
  const context = redactor.redactContext(cloneJson(req.context), undefined);

  const request: RedactedCheckRequest = {
    ...req,
    input,
    proposed_output: proposedOutput,
    context: context as CheckRequest['context'],
    redaction: {
      mode: options.mode,
      status: 'applied',
      entities: redactor.entities(),
      input_redacted: input !== req.input,
      proposed_output_redacted: proposedOutput !== req.proposed_output,
      context_redacted: JSON.stringify(context) !== JSON.stringify(req.context),
    },
  };

  return { request, tokenMap: redactor.tokenMap() };
}

function cloneJson(value: CheckRequest['context']): JsonLike {
  return JSON.parse(JSON.stringify(value ?? {})) as JsonLike;
}

class LocalRedactor {
  private readonly enabled: RedactionEntityType[];
  private readonly rawToToken = new Map<string, string>();
  private readonly tokenToRaw = new Map<string, string>();
  private readonly counts = new Map<string, number>();
  private readonly nextByType = new Map<string, number>();

  constructor(enabled: RedactionEntityType[]) {
    this.enabled = enabled;
  }

  redactContext(value: JsonLike, key: string | undefined): JsonLike {
    if (key !== undefined && CONTEXT_PASSTHROUGH_KEYS.has(key)) return value;
    if (typeof value === 'string') return this.redactText(value);
    if (Array.isArray(value)) return value.map((item) => this.redactContext(item, key));
    if (value !== null && typeof value === 'object') {
      return Object.fromEntries(
        Object.entries(value).map(([childKey, childValue]) => [
          childKey,
          this.redactContext(childValue, childKey),
        ]),
      );
    }
    return value;
  }

  redactText(text: string): string {
    const enabled = new Set(this.enabled);
    return APPLICATION_ORDER.reduce((current, entityType) => {
      if (!enabled.has(entityType)) return current;
      const pattern = PATTERNS[entityType];
      pattern.lastIndex = 0;
      return current.replace(pattern, (raw) => this.tokenFor(entityType, raw));
    }, text);
  }

  entities(): RedactedEntity[] {
    return [...this.counts.entries()].map(([compoundKey, count]) => {
      const [entityType, token] = compoundKey.split('|');
      return { entity_type: entityType!, token: token!, count };
    });
  }

  tokenMap(): Map<string, string> {
    return new Map(this.tokenToRaw);
  }

  private tokenFor(entityType: RedactionEntityType, raw: string): string {
    const rawKey = `${entityType}|${raw}`;
    const existing = this.rawToToken.get(rawKey);
    const token = existing ?? this.createToken(entityType, rawKey, raw);
    const countKey = `${entityType}|${token}`;
    this.counts.set(countKey, (this.counts.get(countKey) ?? 0) + 1);
    return token;
  }

  private createToken(entityType: RedactionEntityType, rawKey: string, raw: string): string {
    const next = this.nextByType.get(entityType) ?? 1;
    this.nextByType.set(entityType, next + 1);
    const token = `[${entityType}_${next}]`;
    this.rawToToken.set(rawKey, token);
    this.tokenToRaw.set(token, raw);
    return token;
  }
}
