import type { Metadata } from 'next';
import { env } from '@/env';
import { GITHUB_URL } from '@/lib/github';

export const SITE_NAME = 'TrustLoopGuard';
export const SITE_URL = stripTrailingSlash(env.NEXT_PUBLIC_SITE_URL);
export const DEFAULT_DESCRIPTION =
  'Check proposed AI agent outputs and actions against policy before execution. Get allow, block, rewrite, or escalate decisions with a reason and trace ID.';

export type LandingSlug =
  | 'ai-agent-spend-controls'
  | 'ai-agent-payment-gateway'
  | 'mcp-spend-guard'
  | 'agentic-travel-payments'
  | 'shopping-agent-checkout'
  | 'accounts-payable-agents'
  | 'ai-agent-audit-trail';

interface LandingFaq {
  question: string;
  answer: string;
}

export interface LandingPageData {
  slug: LandingSlug;
  lastModified: string;
  title: string;
  description: string;
  eyebrow: string;
  h1: string;
  intro: string;
  primaryCta: string;
  secondaryCta: string;
  problemHeading: string;
  problems: string[];
  checkHeading: string;
  checks: Array<{ title: string; body: string }>;
  proofHeading: string;
  proof: string[];
  faq: LandingFaq[];
}

export const landingPages = [
  {
    slug: 'ai-agent-spend-controls',
    lastModified: '2026-07-06',
    title: 'AI Agent Spend Controls | TrustLoopGuard',
    description:
      'Set spend caps for AI agents before they pay, refund, book, or change accounts. TrustLoopGuard returns allow, block, cap, or hold with an audit trail.',
    eyebrow: 'AI agent spend controls',
    h1: 'Stop agent overspend before money moves.',
    intro:
      'TrustLoopGuard adds a pre-action check between an AI agent and a money-moving tool. Your app sends the proposed amount, merchant, action, and context; the guard returns allow, cap, block, or hold before the charge fires.',
    primaryCta: 'Add spend controls',
    secondaryCta: 'Read the quickstart',
    problemHeading: 'The risk is not the answer. It is the action.',
    problems: [
      'A shopping agent retries a checkout link and creates a duplicate order.',
      'A travel agent books a non-refundable itinerary above the user cap.',
      'An AP agent pays an invoice with swapped bank details.',
      'A support agent issues a refund that policy says should be reviewed.',
    ],
    checkHeading: 'What the check sees',
    checks: [
      {
        title: 'Spend limits',
        body: 'Per-transaction, daily, monthly, merchant, and category caps before the action is allowed.',
      },
      {
        title: 'Policy verdicts',
        body: 'Return allow, cap, block, or hold-for-approval with a reason the app can show or log.',
      },
      {
        title: 'Action context',
        body: 'Check the amount, merchant, user intent, workflow step, and source evidence together.',
      },
    ],
    proofHeading: 'Every decision leaves a trail',
    proof: [
      'Trace id for each decision',
      'Policy and cap that fired',
      'Amount, merchant, and action metadata',
      'Reviewer or approval context when a human steps in',
    ],
    faq: [
      {
        question: 'Where does TrustLoopGuard sit?',
        answer:
          'It sits before the tool or payment action fires. Your agent proposes an action, your app checks it with TrustLoopGuard, then applies the returned verdict.',
      },
      {
        question: 'Does this replace payment rails?',
        answer:
          'No. TrustLoopGuard is a policy and audit layer that works with the rails your app already uses.',
      },
    ],
  },
  {
    slug: 'ai-agent-payment-gateway',
    lastModified: '2026-07-06',
    title: 'AI Agent Payment Gateway Controls | TrustLoopGuard',
    description:
      'Put policy checks in front of AI agent payment gateways. TrustLoopGuard gates charges, refunds, bookings, and account actions before they reach the provider.',
    eyebrow: 'AI agent payment gateway',
    h1: 'Add a policy gate before the payment gateway.',
    intro:
      'Provider-compatible gateways make it easy for agents to call models and tools. TrustLoopGuard adds the missing action boundary: a check that evaluates the proposed charge, refund, booking, or account change before it reaches the payment provider.',
    primaryCta: 'Map a gateway check',
    secondaryCta: 'Read gateway docs',
    problemHeading: 'Gateway traffic needs money-action context.',
    problems: [
      'A model request looks normal, but the attached tool action charges a card.',
      'A provider route can forward traffic without knowing the user spend cap.',
      'A refund or order edit needs approval before the gateway completes it.',
      'A payment rail can prove identity while still missing transaction judgment.',
    ],
    checkHeading: 'What to enforce before forwarding',
    checks: [
      {
        title: 'Provider-compatible routing',
        body: 'Keep your existing provider flow while checking money-moving action metadata first.',
      },
      {
        title: 'Per-action policy',
        body: 'Evaluate each charge, refund, booking, or account change against caps and approval rules.',
      },
      {
        title: 'Trace headers',
        body: 'Attach a trace id so the downstream app can connect provider responses to guard decisions.',
      },
    ],
    proofHeading: 'The gateway should leave evidence',
    proof: [
      'Route and provider context',
      'Action metadata checked before forwarding',
      'Allow, block, cap, or hold verdict',
      'Trace id attached to the decision path',
    ],
    faq: [
      {
        question: 'Is TrustLoopGuard a payment processor?',
        answer:
          'No. It is a guard and audit layer before your app calls the payment, checkout, booking, or provider route.',
      },
      {
        question: 'Can this work with model gateways too?',
        answer:
          'Yes, when the gateway has enough action metadata to know what the agent is about to do before it forwards the call.',
      },
    ],
  },
  {
    slug: 'mcp-spend-guard',
    lastModified: '2026-07-06',
    title: 'MCP Spend Guard for AI Agents | TrustLoopGuard',
    description:
      'Add a spend-guard check around MCP tool calls so AI agents can allow, block, or hold payment actions before they execute.',
    eyebrow: 'MCP spend guard',
    h1: 'Gate MCP tool calls before they spend.',
    intro:
      'MCP is where agents ask tools to act. TrustLoopGuard is designed for that boundary: the agent proposes a checkout, payment, refund, booking, or account action, and the guard returns a verdict before the tool runs.',
    primaryCta: 'Talk through MCP',
    secondaryCta: 'Read the docs',
    problemHeading: 'Tool calls need action policy, not just prompt policy.',
    problems: [
      'A tool call can charge a card even if the final chat answer looks harmless.',
      'A browser or commerce tool may use a saved payment method from the user session.',
      'A retry can repeat the same checkout or refund.',
      'A monthly site cap does not explain why this specific transaction was safe.',
    ],
    checkHeading: 'MCP-native control points',
    checks: [
      {
        title: 'Before tool execution',
        body: 'Check the action name, amount, merchant, and workflow step before the MCP tool performs side effects.',
      },
      {
        title: 'Hold instead of guess',
        body: 'Route borderline actions to a person instead of letting the agent decide silently.',
      },
      {
        title: 'Rail-agnostic proof',
        body: 'Keep the policy trace above any single wallet, token, or checkout provider.',
      },
    ],
    proofHeading: 'Built around the tool-call boundary',
    proof: [
      'Action-level verdicts',
      'Policy context for each proposed side effect',
      'Audit records that explain why a transaction was allowed or blocked',
      'A path from soft software gates to harder payment controls later',
    ],
    faq: [
      {
        question: 'Is the MCP server shipped?',
        answer:
          'The core TrustLoopGuard runtime and SDK path exist today. MCP-specific packaging should be implemented when a design partner pulls for it.',
      },
      {
        question: 'Why not only use a wallet cap?',
        answer:
          'Wallet caps limit spend, but they do not always explain the action, evidence, user intent, or policy reason behind a decision.',
      },
    ],
  },
  {
    slug: 'agentic-travel-payments',
    lastModified: '2026-07-06',
    title: 'Guard Agentic Travel Payments | TrustLoopGuard',
    description:
      'Check AI travel bookings before payment. Cap fares, block duplicate bookings, and hold non-refundable itinerary changes for approval.',
    eyebrow: 'Agentic travel payments',
    h1: 'Make the booking agent prove the charge is safe.',
    intro:
      'Travel agents cross from recommendation into irreversible booking fast. TrustLoopGuard checks fare, refundability, passenger intent, and spend cap before the booking or change is allowed.',
    primaryCta: 'Review travel flow',
    secondaryCta: 'See guard modes',
    problemHeading: 'Travel failures become real charges.',
    problems: [
      'The agent books the cheapest fare without showing that it is non-refundable.',
      'A fare changes between recommendation and checkout.',
      'A retry double-books the same itinerary.',
      'A vague "change my flight" request triggers a paid change without confirmation.',
    ],
    checkHeading: 'Travel guardrails to enforce',
    checks: [
      {
        title: 'Fare and cap checks',
        body: 'Compare the final fare against user, trip, or team spend limits before booking.',
      },
      {
        title: 'Refundability holds',
        body: 'Escalate non-refundable or ambiguous itinerary changes for explicit approval.',
      },
      {
        title: 'Duplicate booking detection',
        body: 'Use session and workflow context to catch repeated checkout attempts.',
      },
    ],
    proofHeading: 'A trace for disputed bookings',
    proof: [
      'Original user intent',
      'Fare and itinerary at decision time',
      'Policy that allowed, blocked, or held the booking',
      'Approver and reason for escalated changes',
    ],
    faq: [
      {
        question: 'Can this stop every travel charge?',
        answer:
          'It gates the actions routed through your app or SDK integration. It should be placed before the payment or booking call.',
      },
      {
        question: 'Does this slow the booking path?',
        answer:
          'The check is designed for the runtime path and returns a compact verdict your app can apply immediately.',
      },
    ],
  },
  {
    slug: 'shopping-agent-checkout',
    lastModified: '2026-07-06',
    title: 'Shopping Agent Checkout Controls | TrustLoopGuard',
    description:
      'Guard AI shopping agents before checkout. Confirm item, variant, price, merchant, and duplicate-order risk before payment.',
    eyebrow: 'Shopping agent checkout',
    h1: 'Catch the wrong cart before checkout.',
    intro:
      'Shopping agents turn discovery into payment. TrustLoopGuard checks the item, variant, merchant, price, and retry context before a one-click or agent-fired checkout completes.',
    primaryCta: 'Guard checkout',
    secondaryCta: 'Read the quickstart',
    problemHeading: 'Tiny product mistakes turn into payment disputes.',
    problems: [
      'The agent buys the wrong size, color, or variant.',
      'The product price is stale by the time checkout starts.',
      'A multi-merchant basket partially fails and retries the wrong item.',
      'A resent checkout link creates a duplicate charge.',
    ],
    checkHeading: 'Checkout fields worth gating',
    checks: [
      {
        title: 'Exact item confirmation',
        body: 'Check product id, variant, quantity, merchant, and final price together.',
      },
      {
        title: 'Price drift blocks',
        body: 'Block or hold purchases when the final price exceeds the user stated limit.',
      },
      {
        title: 'Retry awareness',
        body: 'Use trace context to prevent duplicate order attempts from a repeated flow.',
      },
    ],
    proofHeading: 'Proof for the order edge',
    proof: [
      'Cart state at checkout',
      'Final price and merchant',
      'Policy verdict and reason',
      'Trace id for support or dispute review',
    ],
    faq: [
      {
        question: 'Is this a checkout provider?',
        answer:
          'No. TrustLoopGuard checks the proposed checkout action before your app sends it to the checkout or payment provider.',
      },
      {
        question: 'Can it handle multi-merchant carts?',
        answer:
          'The event shape can include action metadata for each checkout step so your app can hold or block risky partial failures.',
      },
    ],
  },
  {
    slug: 'accounts-payable-agents',
    lastModified: '2026-07-06',
    title: 'Accounts Payable Agent Guardrails | TrustLoopGuard',
    description:
      'Add policy checks to AP and procurement agents before invoices, purchase orders, or wires are approved and paid.',
    eyebrow: 'AP and procurement agents',
    h1: 'Do not let an AP agent pay the wrong invoice quietly.',
    intro:
      'AP and procurement agents can move money from inbox to approval to payment. TrustLoopGuard adds a pre-payment decision for invoice amount, vendor, bank details, duplicate risk, and sign-off policy.',
    primaryCta: 'Map an AP check',
    secondaryCta: 'Read policy docs',
    problemHeading: 'Automation needs proof at the approval boundary.',
    problems: [
      'A spoofed invoice matches a real vendor name.',
      'Bank details change after approval.',
      'A duplicate invoice is paid on retry.',
      'An over-policy payment is approved without the required signer.',
    ],
    checkHeading: 'Controls for invoice actions',
    checks: [
      {
        title: 'Vendor and amount policy',
        body: 'Compare the invoice against limits, known vendor context, and required approval paths.',
      },
      {
        title: 'Bank-detail risk',
        body: 'Hold or block payments when destination details change at the edge of payment.',
      },
      {
        title: 'Approval proof',
        body: 'Record who approved the borderline payment and why it satisfied policy.',
      },
    ],
    proofHeading: 'Audit evidence for finance',
    proof: [
      'Invoice amount and vendor',
      'Workflow step and policy threshold',
      'Approval or escalation decision',
      'Traceable reason for audit review',
    ],
    faq: [
      {
        question: 'Does this replace AP approval software?',
        answer:
          'No. It is a runtime policy check for the moment an agent tries to approve, modify, or pay.',
      },
      {
        question: 'Can it work with existing ERPs?',
        answer:
          'Yes, when the AP workflow sends the proposed action to TrustLoopGuard before the ERP or payment call executes.',
      },
    ],
  },
  {
    slug: 'ai-agent-audit-trail',
    lastModified: '2026-07-06',
    title: 'AI Agent Audit Trail for Money Actions | TrustLoopGuard',
    description:
      'Record why AI agents were allowed, blocked, capped, or held before payments, refunds, bookings, and account changes.',
    eyebrow: 'AI agent audit trail',
    h1: 'Prove what the agent did and why it was allowed.',
    intro:
      'When an agent touches money, the hard question is not just whether it worked. It is who authorized the action, what evidence was available, which policy fired, and why the decision was safe.',
    primaryCta: 'See audit flow',
    secondaryCta: 'Read trace docs',
    problemHeading: 'Without a trace, every dispute becomes archaeology.',
    problems: [
      'Finance asks why a charge was approved and no one has the decision context.',
      'Support sees a refund but not the policy that allowed it.',
      'A team cannot tell whether the user approved a borderline action.',
      'The final chat answer omits the unsafe intermediate step.',
    ],
    checkHeading: 'What the trace should capture',
    checks: [
      {
        title: 'Decision context',
        body: 'Record the proposed action, amount, merchant, principal, workflow step, and source evidence.',
      },
      {
        title: 'Policy outcome',
        body: 'Attach the rule, cap, checker, and verdict that shaped the response.',
      },
      {
        title: 'Human review',
        body: 'Preserve the reviewer outcome when a blocked or held action needs a person.',
      },
    ],
    proofHeading: 'Useful audit records are action-level',
    proof: [
      'Allow, cap, block, or hold verdict',
      'Reason shown to the app or operator',
      'Trace id for follow-up',
      'Review outcome for escalated decisions',
    ],
    faq: [
      {
        question: 'Why audit intermediate steps?',
        answer:
          'An agent can take an unsafe action even if the final response looks harmless. Action-level traces preserve the step that mattered.',
      },
      {
        question: 'Is metadata used for enforcement?',
        answer:
          'Metadata is useful for audit context. Enforcement should rely on the event fields and policy inputs your integration treats as authoritative.',
      },
    ],
  },
] as const satisfies readonly LandingPageData[];

export function absoluteUrl(path: string): string {
  if (path === '/') return SITE_URL;
  return `${SITE_URL}${path.startsWith('/') ? path : `/${path}`}`;
}

export function getLandingPage(slug: LandingSlug): LandingPageData {
  const page = landingPages.find((item) => item.slug === slug);
  if (!page) throw new Error(`Unknown landing page: ${slug}`);
  return page;
}

export function buildLandingMetadata(page: LandingPageData): Metadata {
  const path = `/${page.slug}`;
  return {
    title: {
      absolute: page.title,
    },
    description: page.description,
    alternates: {
      canonical: path,
    },
    openGraph: {
      title: page.title,
      description: page.description,
      url: absoluteUrl(path),
      siteName: SITE_NAME,
      type: 'website',
    },
    twitter: {
      card: 'summary_large_image',
      title: page.title,
      description: page.description,
    },
  };
}

export function organizationJsonLd() {
  return {
    '@context': 'https://schema.org',
    '@type': 'Organization',
    name: SITE_NAME,
    url: SITE_URL,
    logo: absoluteUrl('/trustloop-logo.svg'),
    sameAs: [GITHUB_URL],
  };
}

export function serializeJsonLd(value: unknown): string {
  return JSON.stringify(value).replace(/</g, '\\u003c');
}

export function softwareApplicationJsonLd() {
  return {
    '@context': 'https://schema.org',
    '@type': 'SoftwareApplication',
    name: SITE_NAME,
    applicationCategory: 'SecurityApplication',
    operatingSystem: 'Web',
    url: SITE_URL,
    description: DEFAULT_DESCRIPTION,
    offers: {
      '@type': 'Offer',
      price: '0',
      priceCurrency: 'USD',
      availability: 'https://schema.org/PreOrder',
    },
  };
}

export function websiteJsonLd() {
  return {
    '@context': 'https://schema.org',
    '@type': 'WebSite',
    name: SITE_NAME,
    url: SITE_URL,
  };
}

export function landingPageJsonLd(page: LandingPageData) {
  return {
    '@context': 'https://schema.org',
    '@type': 'WebPage',
    name: page.title,
    description: page.description,
    url: absoluteUrl(`/${page.slug}`),
    isPartOf: {
      '@type': 'WebSite',
      name: SITE_NAME,
      url: SITE_URL,
    },
  };
}

export function faqJsonLd(page: LandingPageData) {
  return {
    '@context': 'https://schema.org',
    '@type': 'FAQPage',
    mainEntity: page.faq.map((item) => ({
      '@type': 'Question',
      name: item.question,
      acceptedAnswer: {
        '@type': 'Answer',
        text: item.answer,
      },
    })),
  };
}

function stripTrailingSlash(value: string): string {
  return value.endsWith('/') ? value.slice(0, -1) : value;
}
