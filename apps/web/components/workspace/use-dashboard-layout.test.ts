import { describe, expect, it } from 'vitest';

import { DASHBOARD_WIDGET_KEYS } from './dashboard-widgets';
import { defaultLayout, reconcileLayout } from './use-dashboard-layout';

const ALL_KEYS = [...DASHBOARD_WIDGET_KEYS];

describe('reconcileLayout', () => {
  it('returns the default layout when nothing is stored', () => {
    expect(reconcileLayout(undefined)).toEqual(defaultLayout());
    expect(reconcileLayout(null)).toEqual(defaultLayout());
  });

  it('falls back to the default for malformed values', () => {
    expect(reconcileLayout(42)).toEqual(defaultLayout());
    expect(reconcileLayout('nope')).toEqual(defaultLayout());
    expect(reconcileLayout({ order: 'x', hidden: 5 })).toEqual(defaultLayout());
  });

  it('drops keys that are no longer in the registry', () => {
    const result = reconcileLayout({ order: ['ghost', 'usage'], hidden: ['also-gone'] });
    expect(result.order).not.toContain('ghost');
    expect(result.order[0]).toBe('usage');
    expect(result.hidden).toEqual([]);
  });

  it('appends newly-added widgets after the stored order, visible by default', () => {
    const result = reconcileLayout({ order: ['usage'], hidden: [] });
    expect(result.order[0]).toBe('usage');
    // Every other registry key is present exactly once.
    expect(new Set(result.order)).toEqual(new Set(ALL_KEYS));
    expect(result.order).toHaveLength(ALL_KEYS.length);
  });

  it('preserves a valid stored order and hidden set', () => {
    const stored = { order: ALL_KEYS, hidden: ['metrics'] };
    expect(reconcileLayout(stored)).toEqual({ order: ALL_KEYS, hidden: ['metrics'] });
  });

  it('de-duplicates repeated keys in the stored order', () => {
    const result = reconcileLayout({ order: ['usage', 'usage', 'metrics'], hidden: [] });
    expect(result.order.filter((k) => k === 'usage')).toHaveLength(1);
    expect(result.order).toHaveLength(ALL_KEYS.length);
  });

  it('supports hiding every widget', () => {
    const result = reconcileLayout({ order: ALL_KEYS, hidden: ALL_KEYS });
    expect(result.hidden).toEqual(ALL_KEYS);
    expect(result.order.filter((k) => !result.hidden.includes(k))).toEqual([]);
  });
});
