import { describe, expect, it } from 'vitest';

import { schemaHash } from './canonical-json';

describe('schemaHash', () => {
  it('is stable across nested object insertion order and changes with the schema', () => {
    expect(
      schemaHash({ type: 'object', properties: { b: { type: 'number' }, a: { type: 'string' } } }),
    ).toBe(
      schemaHash({ properties: { a: { type: 'string' }, b: { type: 'number' } }, type: 'object' }),
    );
    expect(schemaHash({ type: 'string' })).not.toBe(schemaHash({ type: 'number' }));
  });
});
