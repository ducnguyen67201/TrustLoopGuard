import { describe, expect, it } from 'vitest';
import { shouldNotify } from './version-watcher';

describe('shouldNotify', () => {
  it('notifies when the server build id differs from the loaded one', () => {
    expect(shouldNotify('abc123', 'def456')).toBe(true);
  });

  it('stays quiet when the build ids match', () => {
    expect(shouldNotify('abc123', 'abc123')).toBe(false);
  });

  it('stays quiet for local dev builds', () => {
    expect(shouldNotify('dev-1', 'dev-2')).toBe(false);
  });

  it('stays quiet when either id is missing', () => {
    expect(shouldNotify(undefined, 'def456')).toBe(false);
    expect(shouldNotify('abc123', null)).toBe(false);
  });
});
