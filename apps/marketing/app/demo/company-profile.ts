import { z } from 'zod';

const slugPattern = /^[a-z0-9]+(?:-[a-z0-9]+)*$/;
const domainPattern = /^(?:[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?\.)+[a-z]{2,63}$/;
const emailPattern = /[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}/i;
const hexColorPattern = /^#[0-9a-f]{6}$/i;

const publicText = (maximum: number) =>
  z
    .string()
    .trim()
    .min(1)
    .max(maximum)
    .refine((value) => !emailPattern.test(value), 'Private contact details are not allowed');

const publicHttpsUrl = z
  .url()
  .max(2_000)
  .refine((value) => {
    const url = new URL(value);
    return url.protocol === 'https:' && url.username === '' && url.password === '';
  }, 'Source must be a public HTTPS URL');

export const demoSlugSchema = z.string().max(100).regex(slugPattern);

export const demoCategorySchema = z.enum(['generic', 'healthcare', 'procurement']);

const fixedDemoCategorySchema = z.enum(['healthcare', 'procurement']);

export const demoScenarioIdByCategory = {
  healthcare: 'healthcare-scheduling-v1',
  procurement: 'procurement-submit-po-v1',
} as const satisfies Record<z.infer<typeof fixedDemoCategorySchema>, string>;

export const demoEffectSchema = z.enum(['permit', 'require_approval', 'deny']);

export const genericContextualScenarioId = 'internal-agent-tool-action-v1' as const;

const demoPathSchema = z
  .object({
    effect: demoEffectSchema,
    label: publicText(80),
    proposal: publicText(1_000),
    evidence: z.array(publicText(300)).min(1).max(6),
    decision: publicText(1_000),
  })
  .strict();

export const outboundDemoProfileSchema = z
  .object({
    slug: demoSlugSchema,
    category: demoCategorySchema,
    company_name: publicText(200),
    company_domain: z.string().max(253).regex(domainPattern).nullable().optional(),
    scenario_id: z.string().max(100).regex(slugPattern),
    demo_url: publicHttpsUrl,
    user_profile: publicText(2_000),
    workflow: publicText(2_000),
    risk_boundary: publicText(2_000),
    rule: publicText(2_000),
    approval_step: publicText(2_000),
    record_shown: publicText(2_000),
    branding: z
      .object({
        primary_color: z.string().regex(hexColorPattern),
        secondary_color: z.string().regex(hexColorPattern),
        tone: publicText(500),
      })
      .strict(),
    paths: z
      .tuple([demoPathSchema, demoPathSchema, demoPathSchema])
      .refine(
        (paths) => new Set(paths.map(({ effect }) => effect)).size === demoEffectSchema.options.length,
        'Paths must contain permit, require_approval, and deny exactly once',
      ),
    sources: z
      .array(
        z
          .object({
            title: publicText(200),
            url: publicHttpsUrl,
          })
          .strict(),
      )
      .min(1)
      .max(12),
    disclaimer: publicText(1_000).refine(
      (value) => value.toLowerCase().includes('concept') && value.toLowerCase().includes('not connected'),
      'Disclaimer must identify the page as a concept that is not connected to the company',
    ),
    truth_check: publicText(2_000),
    local_verification: publicText(2_000),
    live_verified: z.boolean(),
    status: z.enum(['draft', 'active', 'expired']),
    expires_at: z.iso.datetime({ offset: true }).nullable(),
  })
  .strict()
  .superRefine((profile, context) => {
    const url = new URL(profile.demo_url);
    const expectedPath =
      profile.category === 'generic'
        ? `/demo/${profile.slug}`
        : `/demo/${profile.category}/${profile.slug}`;
    if (
      url.origin !== 'https://featherlane.ai' ||
      url.pathname !== expectedPath ||
      url.search !== '' ||
      url.hash !== ''
    ) {
      context.addIssue({
        code: 'custom',
        path: ['demo_url'],
        message: 'Demo URL must match the canonical workflow category route',
      });
    }
    if (
      profile.category !== 'generic' &&
      profile.scenario_id !== demoScenarioIdByCategory[profile.category] &&
      !(
        profile.category === 'healthcare' &&
        profile.scenario_id === genericContextualScenarioId
      )
    ) {
      context.addIssue({
        code: 'custom',
        path: ['scenario_id'],
        message: 'Scenario must match a reviewed runtime for the category',
      });
    }
    if (profile.category === 'generic') {
      if (profile.scenario_id !== genericContextualScenarioId) {
        context.addIssue({
          code: 'custom',
          path: ['scenario_id'],
          message: 'Generic demo scenario must select the reviewed contextual policy pack',
        });
      }
      const companySlugs = [toSlug(profile.company_name), profile.company_domain?.split('.')[0]].filter(
        (value): value is string => Boolean(value),
      );
      if (companySlugs.some((companySlug) => includesSlug(profile.slug, companySlug))) {
        context.addIssue({
          code: 'custom',
          path: ['slug'],
          message: 'Generic demo slug must describe the workflow category, not the company',
        });
      }
    }
  });

export type DemoEffect = z.infer<typeof demoEffectSchema>;
export type DemoCategory = z.infer<typeof demoCategorySchema>;
export type OutboundDemoProfile = z.infer<typeof outboundDemoProfileSchema>;
export type OutboundDemoLocale = 'en' | 'vi';
export type CompanyDemoViewModel = Pick<
  OutboundDemoProfile,
  | 'slug'
  | 'company_name'
  | 'scenario_id'
  | 'user_profile'
  | 'workflow'
  | 'risk_boundary'
  | 'rule'
  | 'approval_step'
  | 'record_shown'
  | 'paths'
  | 'disclaimer'
> & {
  branding: Pick<OutboundDemoProfile['branding'], 'primary_color' | 'secondary_color'>;
};

export type JsonValue =
  | string
  | number
  | boolean
  | null
  | JsonValue[]
  | { [key: string]: JsonValue };

const vietnameseProfileSignals = [
  'trợ lý',
  'không được',
  'được phép',
  'khách hàng',
  'bệnh viện',
] as const;

export function inferOutboundDemoProfileLocale(
  profile: Pick<
    OutboundDemoProfile,
    'workflow' | 'risk_boundary' | 'rule' | 'approval_step' | 'record_shown'
  >,
): OutboundDemoLocale {
  const publicCopy = [
    profile.workflow,
    profile.risk_boundary,
    profile.rule,
    profile.approval_step,
    profile.record_shown,
  ]
    .join(' ')
    .toLocaleLowerCase('vi');
  const matchingSignals = vietnameseProfileSignals.filter((signal) =>
    publicCopy.includes(signal),
  );

  return matchingSignals.length >= 2 ? 'vi' : 'en';
}

export function parseOutboundDemoProfile(value: JsonValue): OutboundDemoProfile | null {
  const result = outboundDemoProfileSchema.safeParse(value);
  return result.success ? result.data : null;
}

function toSlug(value: string): string {
  return value
    .normalize('NFKD')
    .toLowerCase()
    .replace(/\p{M}+/gu, '')
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-|-$/g, '');
}

function includesSlug(slug: string, companySlug: string): boolean {
  return (
    slug === companySlug ||
    slug.startsWith(`${companySlug}-`) ||
    slug.endsWith(`-${companySlug}`) ||
    slug.split('-').includes(companySlug)
  );
}

export function isActiveDemoProfile(
  profile: OutboundDemoProfile,
  now: Date = new Date(),
): boolean {
  if (profile.status !== 'active' || !profile.live_verified) {
    return false;
  }

  return profile.expires_at === null || new Date(profile.expires_at).getTime() > now.getTime();
}
