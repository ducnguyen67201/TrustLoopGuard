import { describe, expect, test } from 'vitest';

import {
  containsPath,
  emptyRegistry,
  findRegistration,
  parseRegistry,
  registeredTargets,
  removeRegistrationTargets,
  RUNTIME_VERSION,
  upsertRegistration,
} from './registry.js';
import type { Registry } from './types.js';

function registryWith(root: string): Registry {
  return upsertRegistration(
    emptyRegistry(),
    {
      root,
      url: 'https://api.example.test',
      agentId: 'agent',
      targets: ['claude', 'codex'],
      cliVersion: '0.0.1',
      runtimeVersion: RUNTIME_VERSION,
    },
    '2026-01-01T00:00:00.000Z',
  );
}

describe('registry', () => {
  test('matches exact and nested roots without sibling-prefix collisions', () => {
    expect(containsPath('/repo/app', '/repo/app')).toBe(true);
    expect(containsPath('/repo/app', '/repo/app/src')).toBe(true);
    expect(containsPath('/repo/app', '/repo/application')).toBe(false);
  });

  test('selects the longest registered root', () => {
    const parent = registryWith('/repo/app');
    const nested = upsertRegistration(
      parent,
      {
        ...parent.projects[0]!,
        root: '/repo/app/packages/api',
        targets: ['opencode'],
      },
      '2026-01-02T00:00:00.000Z',
    );
    expect(findRegistration(nested, '/repo/app/packages/api/src')?.targets).toEqual(['opencode']);
  });

  test('preserves creation time when updating', () => {
    const before = registryWith('/repo/app');
    const after = upsertRegistration(
      before,
      {
        ...before.projects[0]!,
        agentId: 'new-agent',
      },
      '2026-01-03T00:00:00.000Z',
    );
    expect(after.projects[0]?.createdAt).toBe('2026-01-01T00:00:00.000Z');
    expect(after.projects[0]?.updatedAt).toBe('2026-01-03T00:00:00.000Z');
  });

  test('removes selected targets and then the final registration', () => {
    const before = registryWith('/repo/app');
    const partial = removeRegistrationTargets(
      before,
      '/repo/app',
      ['claude'],
      '2026-01-02T00:00:00.000Z',
    );
    expect(partial.projects[0]?.targets).toEqual(['codex']);
    expect([...registeredTargets(partial)]).toEqual(['codex']);
    expect(
      removeRegistrationTargets(partial, '/repo/app', 'all', '2026-01-03T00:00:00.000Z').projects,
    ).toEqual([]);
  });

  test('rejects malformed and unsupported registries', () => {
    expect(() => parseRegistry({ version: 2, projects: [] })).toThrow(/unsupported/);
    expect(() => parseRegistry({ version: 1, projects: 'bad' })).toThrow(/projects/);
    expect(() =>
      parseRegistry({
        version: 1,
        projects: [
          {
            root: '/repo',
            url: 'https://api.example.test',
            agentId: 'a',
            targets: ['other'],
            cliVersion: '1',
            runtimeVersion: '1',
            createdAt: 'now',
            updatedAt: 'now',
          },
        ],
      }),
    ).toThrow(/targets/);
  });
});
