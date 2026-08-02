'use client';

import { useCallback, useEffect, useState } from 'react';
import { DASHBOARD_WIDGET_KEYS, type WidgetKey } from './dashboard-widgets';

export type DashboardLayout = {
  /** Widget keys in display order. */
  order: WidgetKey[];
  /** Widget keys the user has hidden. */
  hidden: WidgetKey[];
};

const STORAGE_PREFIX = 'featherlane-ai:dashboard-layout:';

function storageKey(workspaceSlug: string): string {
  return `${STORAGE_PREFIX}${workspaceSlug}`;
}

/** The default layout: every registered widget, in registry order, all visible. */
export function defaultLayout(): DashboardLayout {
  return { order: [...DASHBOARD_WIDGET_KEYS], hidden: [] };
}

/**
 * Coerce an untrusted stored value into a valid layout for the current widget
 * registry. Drops keys that no longer exist, appends newly-added widgets (visible
 * by default), and falls back to the default for anything malformed. This keeps
 * old saved layouts working when widgets are added or removed later.
 */
export function reconcileLayout(
  stored: unknown,
  registryKeys: readonly WidgetKey[] = DASHBOARD_WIDGET_KEYS,
): DashboardLayout {
  const known = new Set<string>(registryKeys);
  const isKnown = (value: unknown): value is WidgetKey =>
    typeof value === 'string' && known.has(value);

  const record =
    stored && typeof stored === 'object' ? (stored as Record<string, unknown>) : undefined;

  const storedOrder = Array.isArray(record?.['order']) ? record['order'] : [];
  const storedHidden = Array.isArray(record?.['hidden']) ? record['hidden'] : [];

  // Keep valid, de-duplicated stored order, then append any registry keys the
  // stored order is missing (e.g. widgets added since the layout was saved).
  const seen = new Set<WidgetKey>();
  const order: WidgetKey[] = [];
  for (const key of storedOrder) {
    if (isKnown(key) && !seen.has(key)) {
      seen.add(key);
      order.push(key);
    }
  }
  for (const key of registryKeys) {
    if (!seen.has(key)) order.push(key);
  }

  const hidden = [...new Set(storedHidden.filter(isKnown))];

  return { order, hidden };
}

type UseDashboardLayout = {
  /** Widget keys in display order (all widgets, including hidden ones). */
  order: WidgetKey[];
  /** Widget keys that are currently visible, in order. */
  visibleKeys: WidgetKey[];
  isHidden: (key: WidgetKey) => boolean;
  toggle: (key: WidgetKey) => void;
  move: (fromKey: WidgetKey, toKey: WidgetKey) => void;
  reset: () => void;
  isCustomizing: boolean;
  setCustomizing: (value: boolean) => void;
};

/**
 * Per-workspace dashboard layout preference, persisted in localStorage.
 *
 * Layout is a per-user UI preference (like theme), not guardrail data, so it
 * lives client-side. Renders default-first to avoid SSR/hydration mismatch, then
 * hydrates the stored layout after mount.
 */
export function useDashboardLayout(workspaceSlug: string): UseDashboardLayout {
  const [layout, setLayout] = useState<DashboardLayout>(defaultLayout);
  const [isCustomizing, setCustomizing] = useState(false);

  // Hydrate from storage after mount (default-first render matches the server).
  useEffect(() => {
    if (typeof window === 'undefined') return;
    try {
      const raw = window.localStorage.getItem(storageKey(workspaceSlug));
      setLayout(raw ? reconcileLayout(JSON.parse(raw)) : defaultLayout());
    } catch {
      setLayout(defaultLayout());
    }
  }, [workspaceSlug]);

  // Update state and mirror the new layout into storage in one step.
  const update = useCallback(
    (updater: (current: DashboardLayout) => DashboardLayout) => {
      setLayout((current) => {
        const next = updater(current);
        if (next !== current && typeof window !== 'undefined') {
          try {
            window.localStorage.setItem(storageKey(workspaceSlug), JSON.stringify(next));
          } catch {
            // Storage unavailable (private mode / quota) — keep the in-memory layout.
          }
        }
        return next;
      });
    },
    [workspaceSlug],
  );

  const toggle = useCallback(
    (key: WidgetKey) => {
      update((current) => ({
        ...current,
        hidden: current.hidden.includes(key)
          ? current.hidden.filter((k) => k !== key)
          : [...current.hidden, key],
      }));
    },
    [update],
  );

  const move = useCallback(
    (fromKey: WidgetKey, toKey: WidgetKey) => {
      update((current) => {
        const from = current.order.indexOf(fromKey);
        const to = current.order.indexOf(toKey);
        if (from < 0 || to < 0 || from === to) return current;
        const order = [...current.order];
        const [moved] = order.splice(from, 1);
        if (moved === undefined) return current;
        order.splice(to, 0, moved);
        return { ...current, order };
      });
    },
    [update],
  );

  const reset = useCallback(() => {
    if (typeof window !== 'undefined') {
      try {
        window.localStorage.removeItem(storageKey(workspaceSlug));
      } catch {
        // ignore
      }
    }
    setLayout(defaultLayout());
  }, [workspaceSlug]);

  const visibleKeys = layout.order.filter((key) => !layout.hidden.includes(key));
  const isHidden = useCallback((key: WidgetKey) => layout.hidden.includes(key), [layout.hidden]);

  return {
    order: layout.order,
    visibleKeys,
    isHidden,
    toggle,
    move,
    reset,
    isCustomizing,
    setCustomizing,
  };
}
