import type { ReactNode } from 'react';

import { Checkbox } from '@/components/ui/checkbox';
import { cn } from '@/lib/utils';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';

export type DataTableAlign = 'left' | 'right' | 'center';

export interface DataTableColumn<T> {
  id: string;
  header: ReactNode;
  cell: (row: T) => ReactNode;
  align?: DataTableAlign;
  className?: string;
  headerClassName?: string;
  cellClassName?: string;
}

export interface DataTableProps<T> {
  columns: DataTableColumn<T>[];
  rows: T[];
  getRowKey: (row: T) => string;
  selection?: {
    selectedRowKeys: string[];
    onSelectedRowKeysChange: (keys: string[]) => void;
    getRowCanSelect?: (row: T) => boolean;
  };
  empty?: ReactNode;
  caption?: ReactNode;
  className?: string;
}

const alignClass: Record<DataTableAlign, string> = {
  left: 'text-left',
  right: 'text-right',
  center: 'text-center',
};

export function DataTable<T>({
  columns,
  rows,
  getRowKey,
  selection,
  empty,
  caption,
  className,
}: DataTableProps<T>) {
  const selectedKeys = selection?.selectedRowKeys ?? [];
  const selectedKeySet = new Set(selectedKeys);
  const selectableKeys =
    selection === undefined
      ? []
      : rows
          .filter((row) => selection.getRowCanSelect?.(row) ?? true)
          .map((row) => getRowKey(row));
  const selectedVisibleCount = selectableKeys.filter((key) => selectedKeySet.has(key)).length;
  const allVisibleSelected =
    selectableKeys.length > 0 && selectedVisibleCount === selectableKeys.length;
  const someVisibleSelected = selectedVisibleCount > 0 && !allVisibleSelected;
  const columnCount = columns.length + (selection ? 1 : 0);

  function setAllVisible(nextSelected: boolean) {
    if (!selection) return;
    const next = new Set(selectedKeys);
    for (const key of selectableKeys) {
      if (nextSelected) {
        next.add(key);
      } else {
        next.delete(key);
      }
    }
    selection.onSelectedRowKeysChange(Array.from(next));
  }

  function setRowSelected(key: string, nextSelected: boolean) {
    if (!selection) return;
    const next = new Set(selectedKeys);
    if (nextSelected) {
      next.add(key);
    } else {
      next.delete(key);
    }
    selection.onSelectedRowKeysChange(Array.from(next));
  }

  return (
    <Table className={className}>
      {caption ? <caption className="sr-only">{caption}</caption> : null}
      <TableHeader>
        <TableRow>
          {selection ? (
            <TableHead className="w-10">
              <Checkbox
                checked={allVisibleSelected || (someVisibleSelected && 'indeterminate')}
                onCheckedChange={(value) => setAllVisible(value === true)}
                disabled={selectableKeys.length === 0}
                aria-label="Select all visible rows"
              />
            </TableHead>
          ) : null}
          {columns.map((column) => (
            <TableHead
              key={column.id}
              className={cn(
                column.align ? alignClass[column.align] : undefined,
                column.className,
                column.headerClassName,
              )}
            >
              {column.header}
            </TableHead>
          ))}
        </TableRow>
      </TableHeader>
      <TableBody>
        {rows.length === 0 ? (
          <TableRow>
            <TableCell
              colSpan={columnCount}
              className="text-center text-sm text-muted-foreground"
            >
              {empty ?? 'No records yet.'}
            </TableCell>
          </TableRow>
        ) : (
          rows.map((row) => {
            const rowKey = getRowKey(row);
            const rowCanSelect = selection?.getRowCanSelect?.(row) ?? true;
            return (
              <TableRow
                key={rowKey}
                data-state={selectedKeySet.has(rowKey) ? 'selected' : undefined}
              >
                {selection ? (
                  <TableCell className="w-10">
                    <Checkbox
                      checked={selectedKeySet.has(rowKey)}
                      onCheckedChange={(value) => setRowSelected(rowKey, value === true)}
                      disabled={!rowCanSelect}
                      aria-label={`Select ${rowKey}`}
                    />
                  </TableCell>
                ) : null}
                {columns.map((column) => (
                  <TableCell
                    key={column.id}
                    className={cn(
                      column.align ? alignClass[column.align] : undefined,
                      column.className,
                      column.cellClassName,
                    )}
                  >
                    {column.cell(row)}
                  </TableCell>
                ))}
              </TableRow>
            );
          })
        )}
      </TableBody>
    </Table>
  );
}
