'use client';

import { useCallback, useMemo, useState } from 'react';

export function useRowSelection(initialSelectedIds: string[] = []) {
  const [selectedIds, setSelectedIds] = useState<string[]>(initialSelectedIds);
  const selectedIdSet = useMemo(() => new Set(selectedIds), [selectedIds]);
  const clearSelection = useCallback(() => setSelectedIds([]), []);

  return {
    selectedIds,
    selectedIdSet,
    setSelectedIds,
    clearSelection,
  };
}
