import { describe, expect, test } from 'vitest';

import {
  buildAssistantPrompt,
  buildSdkSnippet,
  deriveOnboardingStep,
  traceListSchema,
} from './onboarding';

describe('deriveOnboardingStep', () => {
  test('returns workspace when the user has no workspaces', () => {
    expect(
      deriveOnboardingStep({ workspaceCount: 0, apiKeyCount: 0, hasTraces: false }),
    ).toBe('workspace');
  });

  test('returns connect when a workspace exists but no api keys', () => {
    expect(
      deriveOnboardingStep({ workspaceCount: 1, apiKeyCount: 0, hasTraces: false }),
    ).toBe('connect');
  });

  test('returns verify when keys exist but no traces yet', () => {
    expect(
      deriveOnboardingStep({ workspaceCount: 1, apiKeyCount: 2, hasTraces: false }),
    ).toBe('verify');
  });

  test('returns done once traces exist', () => {
    expect(
      deriveOnboardingStep({ workspaceCount: 1, apiKeyCount: 2, hasTraces: true }),
    ).toBe('done');
  });

  test('returns done when traces exist even if every key was revoked', () => {
    expect(
      deriveOnboardingStep({ workspaceCount: 1, apiKeyCount: 0, hasTraces: true }),
    ).toBe('done');
  });
});

describe('buildSdkSnippet', () => {
  const snippet = buildSdkSnippet({
    baseUrl: 'https://api.example.test',
    agentId: 'support-ai',
  });

  test('interpolates base url and agent id', () => {
    expect(snippet).toContain("'https://api.example.test'");
    expect(snippet).toContain("agentId: 'support-ai'");
  });

  test('references the key only through the TLG_API_KEY env var', () => {
    expect(snippet).toContain('process.env.TLG_API_KEY');
    expect(snippet).not.toContain('tl_live_');
  });

  test('matches the SDK README quick-start structure', () => {
    expect(snippet).toContain("import { Client, guard } from '@trustloopguard/sdk'");
    expect(snippet).toContain('client.withRun(');
    expect(snippet).toContain('onBlock:');
    expect(snippet).toContain('onEscalate:');
  });
});

describe('buildAssistantPrompt', () => {
  const prompt = buildAssistantPrompt({
    baseUrl: 'https://api.example.test',
    agentId: 'support-ai',
  });

  test('is self-contained: install, env vars, guard wiring, first run', () => {
    expect(prompt).toContain('npm install @trustloopguard/sdk');
    expect(prompt).toContain('TLG_URL=https://api.example.test');
    expect(prompt).toContain('TLG_API_KEY=');
    expect(prompt).toContain("'support-ai'");
    expect(prompt).toContain('onBlock');
    expect(prompt).toContain('Run the agent once');
  });

  test('never contains a plaintext key', () => {
    expect(prompt).not.toContain('tl_live_');
  });
});

describe('traceListSchema', () => {
  test('parses a captured wire payload and strips extra fields', () => {
    const wire = {
      traces: [
        {
          trace_id: 'a4f0c2ce-1111-2222-3333-444455556666',
          run_id: null,
          environment_id: 'env_default',
          environment: 'Production',
          domain: 'chat',
          decision: 'allow',
          elapsed_ms: 42,
          latest_review_outcome: null,
          payload: { reason: 'ok' },
          created_at: '2026-07-02T09:00:00Z',
        },
      ],
    };

    const parsed = traceListSchema.parse(wire);
    expect(parsed.traces).toHaveLength(1);
    expect(parsed.traces[0]).toEqual({
      trace_id: 'a4f0c2ce-1111-2222-3333-444455556666',
      decision: 'allow',
      elapsed_ms: 42,
      created_at: '2026-07-02T09:00:00Z',
    });
  });

  test('parses an empty trace list', () => {
    expect(traceListSchema.parse({ traces: [] }).traces).toEqual([]);
  });
});
