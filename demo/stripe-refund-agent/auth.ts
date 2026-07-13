import { timingSafeEqual } from 'node:crypto';

const MIN_PROXY_SECRET_LENGTH = 32;

export function requireRefundDemoProxySecret(
  raw = process.env.REFUND_DEMO_PROXY_SECRET,
): string {
  const secret = raw?.trim();
  if (secret === undefined || secret === '') {
    throw new Error('REFUND_DEMO_PROXY_SECRET is required');
  }
  if (secret.length < MIN_PROXY_SECRET_LENGTH) {
    throw new Error(`REFUND_DEMO_PROXY_SECRET must be at least ${MIN_PROXY_SECRET_LENGTH} characters`);
  }
  return secret;
}

export function isValidRefundDemoAuthorization(
  authorization: string | undefined,
  secret = requireRefundDemoProxySecret(),
): boolean {
  if (authorization === undefined) return false;
  const actual = Buffer.from(authorization);
  const expected = Buffer.from(`Bearer ${secret}`);
  return actual.length === expected.length && timingSafeEqual(actual, expected);
}

export class RefundDemoRequestBudget {
  private count = 0;
  private resetAt = 0;

  constructor(
    private readonly options: {
      maxRequests: number;
      windowMs: number;
    },
  ) {}

  tryAcquire(now = Date.now()): boolean {
    if (this.resetAt <= now) {
      this.count = 0;
      this.resetAt = now + this.options.windowMs;
    }
    if (this.count >= this.options.maxRequests) return false;
    this.count += 1;
    return true;
  }
}
