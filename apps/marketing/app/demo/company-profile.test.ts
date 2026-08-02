import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

import {
  inferOutboundDemoProfileLocale,
  isActiveDemoProfile,
  parseOutboundDemoProfile,
  type JsonValue,
} from './company-profile';

const validProfile = {
  slug: 'acme-cloud',
  category: 'healthcare',
  company_name: 'Acme Cloud',
  company_domain: 'acme.example',
  scenario_id: 'healthcare-scheduling-v1',
  demo_url: 'https://featherlane.ai/demo/healthcare/acme-cloud',
  user_profile: 'Hospital scheduling lead',
  workflow: 'Synthetic appointment scheduling',
  risk_boundary: 'A scheduling assistant must stop unsafe requests before drafting a reply.',
  rule: 'Only synthetic, non-clinical scheduling requests may proceed.',
  approval_step: 'Hospital staff review requests that require human judgment.',
  record_shown: 'Input decision, output decision, policy findings, and trace identifiers.',
  branding: {
    primary_color: '#175CD3',
    secondary_color: '#84ADFF',
    tone: 'Clear and operational.',
  },
  paths: [
    {
      effect: 'permit',
      label: 'Allow',
      proposal: 'Explain how to request a fictional primary-care appointment.',
      evidence: ['The request contains no real patient data.', 'The request is non-clinical.'],
      decision: 'The scheduling guidance can be drafted and checked before delivery.',
    },
    {
      effect: 'require_approval',
      label: 'Require approval',
      proposal: 'Change a fictional appointment without confirming the requested details.',
      evidence: ['The request is administrative.', 'Required scheduling details are missing.'],
      decision: 'The request is held for hospital staff review.',
    },
    {
      effect: 'deny',
      label: 'Block',
      proposal: "Reveal another fictional patient's appointment details.",
      evidence: ['The request asks for information about another patient.'],
      decision: 'The request is blocked before model generation.',
    },
  ],
  sources: [{ title: 'Acme scheduling overview', url: 'https://acme.example/scheduling' }],
  disclaimer:
    'This is a concept based on public material and is not connected to Acme Cloud or its systems.',
  truth_check: 'The concept is limited to the workflow described by the linked public source.',
  local_verification: 'Schema, route contract, type-check, and responsive render verified.',
  live_verified: true,
  status: 'active',
  expires_at: '2027-07-19T00:00:00.000Z',
} satisfies JsonValue;

test('accepts a complete public demo profile with all three decision paths', () => {
  const profile = parseOutboundDemoProfile(validProfile);

  assert.equal(profile?.slug, 'acme-cloud');
  assert.deepEqual(
    profile?.paths.map(({ effect }) => effect),
    ['permit', 'require_approval', 'deny'],
  );
});

test('accepts a company-neutral workflow category route for a generic concept', () => {
  const profile = parseOutboundDemoProfile({
    ...validProfile,
    slug: 'cloud-storage-security',
    category: 'generic',
    scenario_id: 'internal-agent-tool-action-v1',
    demo_url: 'https://featherlane.ai/demo/cloud-storage-security',
  });

  assert.equal(profile?.slug, 'cloud-storage-security');
  assert.equal(profile?.category, 'generic');
});

test('accepts the reviewed contextual policy pack under a healthcare category route', () => {
  const profile = parseOutboundDemoProfile({
    ...validProfile,
    slug: 'acme-health-ai-security',
    scenario_id: 'internal-agent-tool-action-v1',
    demo_url: 'https://featherlane.ai/demo/healthcare/acme-health-ai-security',
  });

  assert.equal(profile?.category, 'healthcare');
  assert.equal(profile?.scenario_id, 'internal-agent-tool-action-v1');
});

test('rejects an unreviewed contextual policy pack for a generic concept', () => {
  assert.equal(
    parseOutboundDemoProfile({
      ...validProfile,
      slug: 'cloud-storage-security',
      category: 'generic',
      scenario_id: 'invented-customer-policy-v1',
      demo_url: 'https://featherlane.ai/demo/cloud-storage-security',
    }),
    null,
  );
});

test('rejects a profile with a mismatched route or private outreach data', () => {
  assert.equal(
    parseOutboundDemoProfile({
      ...validProfile,
      demo_url: 'https://featherlane.ai/demo/healthcare/another-company',
    }),
    null,
  );
  assert.equal(parseOutboundDemoProfile({ ...validProfile, category: 'finance' }), null);
  assert.equal(
    parseOutboundDemoProfile({ ...validProfile, scenario_id: 'procurement-submit-po-v1' }),
    null,
  );
  assert.equal(
    parseOutboundDemoProfile({
      ...validProfile,
      recipient_email: 'buyer@acme.example',
    }),
    null,
  );
  assert.equal(
    parseOutboundDemoProfile({
      ...validProfile,
      category: 'generic',
      scenario_id: 'internal-agent-tool-action-v1',
      demo_url: 'https://featherlane.ai/demo/acme-cloud',
    }),
    null,
  );
  assert.equal(
    parseOutboundDemoProfile({
      ...validProfile,
      slug: 'acme-cloud-security',
      category: 'generic',
      scenario_id: 'internal-agent-tool-action-v1',
      demo_url: 'https://featherlane.ai/demo/acme-cloud-security',
    }),
    null,
  );
});

test('rejects incomplete decision paths and unsafe branding values', () => {
  assert.equal(
    parseOutboundDemoProfile({
      ...validProfile,
      paths: validProfile.paths.slice(0, 2),
    }),
    null,
  );
  assert.equal(
    parseOutboundDemoProfile({
      ...validProfile,
      branding: { ...validProfile.branding, primary_color: 'url(javascript:alert(1))' },
    }),
    null,
  );
  assert.equal(
    parseOutboundDemoProfile({
      ...validProfile,
      branding: { ...validProfile.branding, logo_url: 'https://acme.example/logo.svg' },
    }),
    null,
  );
});

test('only activates live-verified, active, unexpired profiles', () => {
  const profile = parseOutboundDemoProfile(validProfile);
  assert.ok(profile);
  assert.equal(isActiveDemoProfile(profile, new Date('2026-07-19T00:00:00.000Z')), true);
  assert.equal(isActiveDemoProfile(profile, new Date('2028-07-19T00:00:00.000Z')), false);

  const draft = parseOutboundDemoProfile({ ...validProfile, status: 'draft' });
  assert.ok(draft);
  assert.equal(isActiveDemoProfile(draft), false);
});

test('infers Vietnamese personalized demos from their public workflow copy', () => {
  assert.equal(
    inferOutboundDemoProfileLocale({
      workflow: 'Trợ lý AI hỗ trợ tra cứu và đổi lịch tái khám cho khách hàng.',
      risk_boundary: 'Trợ lý không được tự đổi lịch.',
      rule: 'Thông tin công khai được phép đọc.',
      approval_step: 'Nhân viên bệnh viện duyệt thay đổi.',
      record_shown: 'Lưu quyết định cuối cùng.',
    }),
    'vi',
  );
  assert.equal(
    inferOutboundDemoProfileLocale({
      workflow: validProfile.workflow,
      risk_boundary: validProfile.risk_boundary,
      rule: validProfile.rule,
      approval_step: validProfile.approval_step,
      record_shown: validProfile.record_shown,
    }),
    'en',
  );
});

test('the workflow category route reads only a generic profile and hides research links', () => {
  const store = readFileSync(
    new URL('../../lib/server/outbound-demo-profile-store.ts', import.meta.url),
    'utf8',
  );
  const page = readFileSync(new URL('./[category]/page.tsx', import.meta.url), 'utf8');
  const demo = readFileSync(new URL('./[category]/company-demo.tsx', import.meta.url), 'utf8');
  const contextualContent = readFileSync(
    new URL('./contextual-content.ts', import.meta.url),
    'utf8',
  );
  const contextualRoute = readFileSync(
    new URL('../api/demo/contextual/[category]/route.ts', import.meta.url),
    'utf8',
  );

  assert.match(store, /WHERE category = \$\{parsedCategory\.data\}/);
  assert.match(store, /AND slug = \$\{parsedSlug\.data\}/);
  assert.doesNotMatch(store, /status = 'active'/);
  assert.doesNotMatch(store, /live_verified = TRUE/);
  assert.doesNotMatch(store, /expires_at > NOW\(\)/);
  assert.match(store, /rows\.length !== 1/);
  assert.match(store, /WHERE category = 'generic'/);
  assert.match(store, /export const getContextualDemoProfile/);
  assert.match(store, /AND scenario_id = \$\{genericContextualScenarioId\}/);
  assert.match(contextualRoute, /getContextualDemoProfile/);
  assert.doesNotMatch(contextualRoute, /getProfile: getGenericDemoProfile/);
  assert.match(page, /getGenericDemoProfile\(category\)/);
  assert.match(page, /notFound\(\)/);
  assert.match(page, /index: false, follow: false/);
  assert.match(contextualContent, /Send through Featherlane AI/);
  assert.match(contextualContent, /Featherlane AI policy monitor/);
  assert.match(contextualContent, /Shared demo workspace/);
  assert.match(demo, /fetch\(endpoint/);
  assert.match(demo, /JSON\.stringify\(\{ locale, sessionId, message: submittedMessage, history \}\)/);
  assert.match(demo, /profile\.company_name/);
  assert.doesNotMatch(demo, /logo_url/);
  assert.match(demo, /profile\.disclaimer/);
  assert.doesNotMatch(demo, /profile\.sources\.map/);
  assert.doesNotMatch(demo, /\/v1\/events/);
  assert.doesNotMatch(demo, /JSON\.stringify\([^\n]*(profile|policyIds|scenarioId)/);
});

test('the personalized healthcare route selects the fixed or reviewed contextual scenario', () => {
  const page = readFileSync(new URL('./healthcare/[company]/page.tsx', import.meta.url), 'utf8');
  const healthcarePage = readFileSync(
    new URL('./healthcare/healthcare-page.tsx', import.meta.url),
    'utf8',
  );
  const contextualPage = readFileSync(
    new URL('./personalized-contextual-page.tsx', import.meta.url),
    'utf8',
  );

  assert.match(page, /getDemoProfile\('healthcare', company\)/);
  assert.match(page, /demoScenarioIdByCategory\.healthcare/);
  assert.match(page, /genericContextualScenarioId/);
  assert.match(page, /PersonalizedContextualDemoPageContent/);
  assert.match(contextualPage, /CompanyDemo/);
  assert.match(page, /notFound\(\)/);
  assert.match(page, /inferOutboundDemoProfileLocale\(profile\) === 'vi'/);
  assert.match(page, /permanentRedirect\(`\/vi\/demo\/healthcare\/\$\{profile\.slug\}`\)/);
  assert.match(page, /index: false, follow: false/);
  assert.match(page, /HealthcareDemoPageContent locale="en" profile={profile}/);
  assert.match(healthcarePage, /profile\?\.risk_boundary/);
  assert.doesNotMatch(healthcarePage, /logo_url/);
  assert.doesNotMatch(healthcarePage, /profile\.sources\.map/);
});

test('the personalized procurement route selects only the fixed purchase-order scenario', () => {
  const page = readFileSync(new URL('./procurement/[company]/page.tsx', import.meta.url), 'utf8');
  const procurementPage = readFileSync(
    new URL('./procurement/procurement-page.tsx', import.meta.url),
    'utf8',
  );

  assert.match(page, /getDemoProfile\('procurement', company\)/);
  assert.match(page, /demoScenarioIdByCategory\.procurement/);
  assert.match(page, /notFound\(\)/);
  assert.match(page, /index: false, follow: false/);
  assert.match(page, /ProcurementDemoPageContent locale="en" profile={profile}/);
  assert.match(procurementPage, /profile\?\.risk_boundary/);
  assert.doesNotMatch(procurementPage, /logo_url/);
  assert.doesNotMatch(procurementPage, /profile\.sources\.map/);
});
