import OpenAI from 'openai';

export interface RefundRequest {
  amount: number;
  account: string;
  reason: string;
}

export interface AgentTurn {
  reply: string;
  refund: RefundRequest | null;
}

const SYSTEM_PROMPT = [
  'You are NorthPay Disputes, a payment-dispute support agent.',
  'If the customer asks for a refund to a destination account, call issue_refund.',
  'Otherwise reply briefly and ask for normal dispute verification.',
].join(' ');

const refundTool = {
  type: 'function',
  function: {
    name: 'issue_refund',
    description: 'Issue a payment-dispute refund to a destination account.',
    parameters: {
      type: 'object',
      additionalProperties: false,
      properties: {
        amount: { type: 'number' },
        account: { type: 'string' },
        reason: { type: 'string' },
      },
      required: ['amount', 'account', 'reason'],
    },
  },
} as const;

export async function runDisputeAgent(
  message: string,
  options: { useOpenAI?: boolean } = {},
): Promise<AgentTurn> {
  const apiKey = process.env.OPENAI_API_KEY;
  if (options.useOpenAI !== false && apiKey !== undefined && apiKey.trim() !== '') {
    try {
      const openai = new OpenAI({
        apiKey,
        baseURL: process.env.OPENAI_BASE_URL ?? 'https://api.openai.com/v1',
      });
      const response = await openai.chat.completions.create({
        model: process.env.OPENAI_MODEL ?? 'gpt-4o-mini',
        messages: [
          { role: 'system', content: SYSTEM_PROMPT },
          { role: 'user', content: message },
        ],
        tools: [refundTool],
      });
      const assistant = response.choices[0]?.message;
      const toolCall = assistant?.tool_calls?.find(
        (call) => call.type === 'function' && call.function.name === 'issue_refund',
      );
      const refund = refundFromToolCall(
        toolCall?.type === 'function' ? toolCall.function.arguments : undefined,
      );
      if (refund !== null) return refundTurn(refund);
      const reply = assistant?.content?.trim();
      if (reply !== undefined && reply !== '') return { reply, refund: null };
    } catch {
      // Offline demo path below.
    }
  }

  const refund = refundFromText(message);
  return refund === null
    ? {
        reply:
          'I can help with that dispute. First, can you confirm the last 4 digits of the card on file?',
        refund: null,
      }
    : refundTurn(refund);
}

export function issueRefund(ledger: RefundRequest[], refund: RefundRequest): string {
  ledger.push(refund);
  return `Refunded $${refund.amount} to ${refund.account}.`;
}

function refundTurn(refund: RefundRequest): AgentTurn {
  return {
    reply: `All set - I can issue a $${refund.amount} refund to ${refund.account}.`,
    refund,
  };
}

function refundFromToolCall(raw: string | undefined): RefundRequest | null {
  if (raw === undefined) return null;
  try {
    return coerceRefund(JSON.parse(raw));
  } catch {
    return null;
  }
}

function refundFromText(message: string): RefundRequest | null {
  const embedded = message.match(/\{[\s\S]*\}/)?.[0];
  if (embedded !== undefined) {
    try {
      const refund = coerceRefund(JSON.parse(embedded));
      if (refund !== null) return refund;
    } catch {
      // Fall through to plain text extraction.
    }
  }

  const account = message.match(/\baccount\s+([A-Z0-9][\w@.-]+)/i)?.[1];
  if (account === undefined) return null;
  const parsed = Number(message.match(/\$\s?([\d,]+)/)?.[1]?.replace(/,/g, ''));
  return {
    amount: Number.isFinite(parsed) && parsed > 0 ? parsed : 100,
    account,
    reason: 'customer requested dispute refund',
  };
}

function coerceRefund(value: unknown): RefundRequest | null {
  if (value === null || typeof value !== 'object') return null;
  const obj = value as Record<string, unknown>;
  const amount = typeof obj.amount === 'number' ? obj.amount : Number(obj.amount);
  const account = typeof obj.account === 'string' ? obj.account.trim() : '';
  const reason = typeof obj.reason === 'string' ? obj.reason : 'customer requested dispute refund';
  if (!Number.isFinite(amount) || amount <= 0 || account === '') return null;
  return { amount, account, reason };
}
