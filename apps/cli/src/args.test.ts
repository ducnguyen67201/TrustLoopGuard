import { describe, expect, test } from 'vitest';

import { parseCliArgs } from './args.js';

describe('parseCliArgs', () => {
  test('parses the default install command', () => {
    expect(parseCliArgs(['install', '--agent-id', 'agent-1'], '/repo')).toEqual({
      command: 'install',
      project: '/repo',
      json: false,
      agentId: 'agent-1',
      allowUnsupported: false,
      target: 'auto',
      url: undefined,
    });
  });

  test('parses a unique host subset', () => {
    const result = parseCliArgs(
      ['install', '--agent-id', 'a', '--target', 'claude,codex', '--json'],
      '/repo',
    );
    expect(result).toMatchObject({ target: ['claude', 'codex'], json: true });
  });

  test('rejects duplicate options and host names', () => {
    expect(() => parseCliArgs(['install', '--agent-id', 'a', '--agent-id', 'b'], '/repo')).toThrow(
      /only once/,
    );
    expect(() =>
      parseCliArgs(['install', '--agent-id', 'a', '--target', 'claude,claude'], '/repo'),
    ).toThrow(/unique/);
  });

  test('rejects API keys and command-specific options', () => {
    expect(() =>
      parseCliArgs(['install', '--agent-id', 'a', '--api-key', 'secret'], '/repo'),
    ).toThrow(/api-key/);
    expect(() => parseCliArgs(['status', '--target', 'claude'], '/repo')).toThrow(
      /not valid with status/,
    );
  });

  test('rejects ambiguous uninstall selection', () => {
    expect(() => parseCliArgs(['uninstall', '--all', '--target', 'claude'], '/repo')).toThrow(
      /cannot be used together/,
    );
  });

  test('returns help for no arguments or help', () => {
    expect(parseCliArgs([], '/repo')).toEqual({ command: 'help' });
    expect(parseCliArgs(['--help'], '/repo')).toEqual({ command: 'help' });
  });
});
