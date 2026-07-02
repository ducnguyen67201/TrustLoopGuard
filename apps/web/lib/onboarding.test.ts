import { describe, expect, test } from 'vitest';

import {
  assistantOptions,
  buildAssistantPrompt,
  buildSdkSnippet,
  createApiKeyResponseSchema,
  sanitizeAgentId,
  traceListSchema,
} from './onboarding';

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
    assistant: 'claude',
  });

  test('is self-contained: install, env vars, guard wiring, first run', () => {
    expect(prompt).toContain('npm install @trustloopguard/sdk');
    expect(prompt).toContain('TLG_URL=https://api.example.test');
    expect(prompt).toContain('TLG_API_KEY=');
    expect(prompt).toContain("'support-ai'");
    expect(prompt).toContain('onBlock');
    expect(prompt).toContain('Run the agent once');
    expect(prompt).toContain('Claude Code');
  });

  test('never contains a plaintext key', () => {
    expect(prompt).not.toContain('tl_live_');
  });

  test('has a specific prompt path for every selectable assistant', () => {
    for (const option of assistantOptions) {
      const tailored = buildAssistantPrompt({
        baseUrl: 'https://api.example.test',
        agentId: 'support-ai',
        assistant: option.id,
      });
      expect(tailored).toContain(option.label);
      expect(tailored).toContain('TLG_API_KEY=');
    }
  });
});

describe('sanitizeAgentId', () => {
  test('collapses runs of disallowed characters into single dashes', () => {
    expect(sanitizeAgentId("billing bot'; drop")).toBe('billing-bot-drop');
    expect(sanitizeAgentId('Support_AI-2')).toBe('Support_AI-2');
    expect(sanitizeAgentId('`${evil}`')).toBe('-evil-');
  });
});

describe('createApiKeyResponseSchema', () => {
  test('parses the create-key wire payload', () => {
    const parsed = createApiKeyResponseSchema.parse({
      api_key: { id: 'key_1', name: 'support-ai key', prefix: 'tl_live_abc' },
      plaintext_key: 'tl_live_abc123',
    });
    expect(parsed.plaintext_key).toBe('tl_live_abc123');
  });

  test('rejects a payload missing the secret', () => {
    expect(() =>
      createApiKeyResponseSchema.parse({
        api_key: { id: 'key_1', name: 'n', prefix: 'p' },
      }),
    ).toThrow();
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
