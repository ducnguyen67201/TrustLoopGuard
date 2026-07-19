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
  company_name: 'Acme Cloud',
  company_domain: 'acme.example',
  scenario_id: 'production-data-export',
  demo_url: 'https://gettrustloop.app/demo/acme-cloud',
  user_profile: 'Operations lead',
  workflow: 'Customer data export',
  risk_boundary: 'An agent is preparing to export production customer records.',
  rule: 'Exports require a documented purpose and approval before execution.',
  approval_step: 'The data owner reviews the request before the export tool runs.',
  record_shown: 'Proposal, evidence, policy result, approval state, and final execution state.',
  branding: {
    primary_color: '#175CD3',
    secondary_color: '#84ADFF',
    tone: 'Clear and operational.',
  },
  paths: [
    {
      effect: 'permit',
      label: 'Allow',
      proposal: 'Export an approved aggregate report.',
      evidence: ['The request is aggregate-only.', 'A current approval is attached.'],
      decision: 'The export can proceed and the authorization is recorded.',
    },
    {
      effect: 'require_approval',
      label: 'Require approval',
      proposal: 'Export customer-level records for a support investigation.',
      evidence: ['The purpose is documented.', 'The data owner has not approved the request.'],
      decision: 'The action is held until the data owner approves it.',
    },
    {
      effect: 'deny',
      label: 'Block',
      proposal: 'Export all customer records to an unapproved destination.',
      evidence: ['The destination is outside the approved list.'],
      decision: 'The export is blocked before the data tool is called.',
    },
  ],
  sources: [{ title: 'Acme security overview', url: 'https://acme.example/security' }],
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
      demo_url: 'https://gettrustloop.app/demo/another-company',
    }),
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

  assert.match(store, /WHERE slug = \$\{parsedSlug\.data\}/);
  assert.match(store, /status = 'active'/);
  assert.match(store, /live_verified = TRUE/);
  assert.match(store, /expires_at > NOW\(\)/);
  assert.match(page, /notFound\(\)/);
  assert.match(page, /index: false, follow: false/);
  assert.match(demo, /Choose a decision path/);
  assert.match(demo, /profile\.disclaimer/);
  assert.doesNotMatch(demo, /fetch\(|\/v1\/events/);
});
