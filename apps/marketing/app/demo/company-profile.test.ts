import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

import {
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
  demo_url: 'https://gettrustloop.app/demo/healthcare/acme-cloud',
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

test('rejects a profile with a mismatched route or private outreach data', () => {
  assert.equal(
    parseOutboundDemoProfile({
      ...validProfile,
      demo_url: 'https://gettrustloop.app/demo/healthcare/another-company',
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

test('the company route reads only eligible profiles and fails closed', () => {
  const store = readFileSync(
    new URL('../../lib/server/outbound-demo-profile-store.ts', import.meta.url),
    'utf8',
  );
  const page = readFileSync(new URL('./[company]/page.tsx', import.meta.url), 'utf8');
  const demo = readFileSync(new URL('./[company]/company-demo.tsx', import.meta.url), 'utf8');

  assert.match(store, /WHERE category = \$\{parsedCategory\.data\}/);
  assert.match(store, /AND slug = \$\{parsedSlug\.data\}/);
  assert.match(store, /status = 'active'/);
  assert.match(store, /live_verified = TRUE/);
  assert.match(store, /expires_at > NOW\(\)/);
  assert.match(store, /rows\.length !== 1/);
  assert.match(page, /notFound\(\)/);
  assert.match(page, /index: false, follow: false/);
  assert.match(demo, /Choose a decision path/);
  assert.match(demo, /profile\.disclaimer/);
  assert.doesNotMatch(demo, /fetch\(|\/v1\/events/);
});

test('the personalized healthcare route selects only the fixed scheduling scenario', () => {
  const page = readFileSync(new URL('./healthcare/[company]/page.tsx', import.meta.url), 'utf8');
  const healthcarePage = readFileSync(
    new URL('./healthcare/healthcare-page.tsx', import.meta.url),
    'utf8',
  );

  assert.match(page, /getActiveDemoProfile\('healthcare', company\)/);
  assert.match(page, /demoScenarioIdByCategory\.healthcare/);
  assert.match(page, /notFound\(\)/);
  assert.match(page, /index: false, follow: false/);
  assert.match(page, /HealthcareDemoPageContent locale="en" profile={profile}/);
  assert.match(healthcarePage, /profile\?\.risk_boundary/);
  assert.match(healthcarePage, /profile\.sources/);
});
