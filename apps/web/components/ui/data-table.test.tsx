import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';

import { DataTable, type DataTableColumn } from './data-table';

type Row = {
  id: string;
  label: string;
  status: 'active' | 'archived';
};

const columns: DataTableColumn<Row>[] = [
  { id: 'label', header: 'Name', cell: (row) => row.label },
  { id: 'status', header: 'Status', cell: (row) => row.status },
];

const rows: Row[] = [
  { id: 'policies', label: 'Policies', status: 'active' },
  { id: 'agents', label: 'Agents', status: 'active' },
  { id: 'settings', label: 'Settings', status: 'archived' },
];

describe('DataTable', () => {
  it('renders an accessible empty state across the table columns', () => {
    render(
      <DataTable
        caption="Policies"
        columns={columns}
        rows={[]}
        getRowKey={(row) => row.id}
        empty="No policies found."
      />,
    );

    expect(screen.getByRole('table', { name: 'Policies' })).toBeInTheDocument();
    expect(screen.getByText('No policies found.')).toBeInTheDocument();
    expect(screen.getByText('No policies found.')).toHaveAttribute('colspan', '2');
  });

  it('keeps row selection scoped to selectable visible rows', async () => {
    const user = userEvent.setup();
    const onSelectedRowKeysChange = vi.fn<(keys: string[]) => void>();

    render(
      <DataTable
        caption="Workspace resources"
        columns={columns}
        rows={rows}
        getRowKey={(row) => row.id}
        selection={{
          selectedRowKeys: ['policies'],
          onSelectedRowKeysChange,
          getRowCanSelect: (row) => row.status === 'active',
        }}
      />,
    );

    expect(screen.getByRole('row', { name: /policies active/i })).toHaveAttribute(
      'data-state',
      'selected',
    );
    expect(screen.getByRole('checkbox', { name: 'Select settings' })).toBeDisabled();

    await user.click(screen.getByRole('checkbox', { name: 'Select all visible rows' }));

    expect(onSelectedRowKeysChange).toHaveBeenCalledWith(['policies', 'agents']);
  });
});
