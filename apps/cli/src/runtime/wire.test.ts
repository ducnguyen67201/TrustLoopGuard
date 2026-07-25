import { describe, expect, test } from 'vitest';

import type { JsonValue } from './runtime-types.js';
import {
  isObject,
  parseApprovalStatus,
  parseCommandHookPayload,
  parseDecision,
  parseRuntimeRegistration,
} from './wire.js';

describe('runtime wire validation', () => {
  test.each([
    ['PreToolUse', 'pre'],
    ['PostToolUse', 'post-success'],
    ['PostToolUseFailure', 'post-failure'],
    ['Stop', 'session-end'],
    ['SessionEnd', 'session-end'],
  ] as const)('maps %s to %s', (hookEventName, expected) => {
    expect(
      parseCommandHookPayload(
        {
          hookEventName,
          toolName: 'Bash',
          callID: 'call-1',
          sessionId: 'session-1',
          projectDir: '/project',
          toolInput: { command: 'true' },
          hostVersion: '1.2.3',
        },
        'codex',
        '/fallback',
      ),
    ).toMatchObject({
      event: expected,
      cwd: '/fallback',
      projectRoot: '/project',
      input: { command: 'true' },
    });
  });

  test('uses conservative payload defaults and rejects invalid events', () => {
    expect(
      parseCommandHookPayload(
        {
          hook_event_name: 'PreToolUse',
          cwd: '/project',
          tool_input: 'not-an-object',
        },
        'claude',
        '/fallback',
      ),
    ).toMatchObject({
      toolName: 'unknown_tool',
      callId: '',
      sessionId: '',
      projectRoot: '/project',
      input: {},
    });
    expect(() => parseCommandHookPayload([], 'claude', '/project')).toThrow(/JSON object/);
    expect(() =>
      parseCommandHookPayload({ hook_event_name: 'UnknownHook' }, 'claude', '/project'),
    ).toThrow(/unsupported/);
  });

  test.each(['permit', 'deny', 'transform', 'require_approval', 'defer'] as const)(
    'accepts the %s decision effect',
    (effect) => {
      expect(parseDecision({ effect })).toEqual({
        effect,
        reason: 'the guard returned no reason',
        traceId: 'n/a',
      });
    },
  );

  test('parses optional decision metadata and validates malformed decisions', () => {
    expect(
      parseDecision({
        effect: 'permit',
        reason: 'allowed',
        trace_id: 'trace-1',
        approval: { id: 'approval-1', poll_after_ms: 0 },
        lease: { id: 'lease-1' },
      }),
    ).toEqual({
      effect: 'permit',
      reason: 'allowed',
      traceId: 'trace-1',
      approval: { id: 'approval-1', pollAfterMs: 1_000 },
      lease: { id: 'lease-1' },
    });
    expect(() => parseDecision([])).toThrow(/non-object/);
    expect(() => parseDecision({ effect: 'unexpected' })).toThrow(/unexpected/);
  });

  test.each(['approved', 'canceled', 'denied', 'expired', 'pending'] as const)(
    'accepts the %s approval status',
    (status) => {
      expect(parseApprovalStatus({ status })).toEqual({ status });
    },
  );

  test('parses approval grants and rejects invalid approval responses', () => {
    expect(parseApprovalStatus({ status: 'approved', grant_id: 'grant-1' })).toEqual({
      status: 'approved',
      grantId: 'grant-1',
    });
    expect(() => parseApprovalStatus(null)).toThrow(/non-object/);
    expect(() => parseApprovalStatus({ status: 'unknown' })).toThrow(/unexpected/);
  });

  test('validates runtime registrations and object guards', () => {
    const registration = {
      root: '/project',
      url: 'https://api.example.test',
      agentId: 'agent',
      targets: ['claude', 'codex'],
      cliVersion: '1',
      runtimeVersion: '1',
      createdAt: 'now',
      updatedAt: 'now',
    };
    expect(parseRuntimeRegistration(registration, 'registry')).toEqual(registration);
    expect(() =>
      parseRuntimeRegistration({ ...registration, targets: 'claude' }, 'registry'),
    ).toThrow(/targets/);
    expect(() =>
      parseRuntimeRegistration({ ...registration, targets: ['other'] }, 'registry'),
    ).toThrow(/targets/);
    expect(() => parseRuntimeRegistration({ ...registration, root: '' }, 'registry')).toThrow(
      /root/,
    );
    expect(isObject(undefined)).toBe(false);
    expect(isObject({})).toBe(true);
  });
});

test('the wire fixtures remain JSON values', () => {
  const value: JsonValue = { safe: true };
  expect(isObject(value)).toBe(true);
});
