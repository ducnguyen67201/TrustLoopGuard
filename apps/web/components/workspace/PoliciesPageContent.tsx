'use client';

import { IconPlus } from '@tabler/icons-react';

import { Badge } from '@/components/ui/badge';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { DataTable, type DataTableColumn } from '@/components/ui/data-table';
import { PolicyCreateDialog } from '@/components/workspace/PolicyCreateDialog';
import type { AgentRow, DashboardShellData, PolicyRow } from '@/lib/server/dashboard-data';

type PoliciesPageData = DashboardShellData & { agents: AgentRow[]; policies: PolicyRow[] };

const policyColumns: DataTableColumn<PolicyRow>[] = [
  {
    id: 'id',
    header: 'Policy',
    cell: (row) => row.id,
    cellClassName: 'font-mono text-xs',
  },
  {
    id: 'description',
    header: 'Description',
    cell: (row) => row.description,
    cellClassName: 'text-muted-foreground',
  },
  { id: 'agent', header: 'Agent', cell: (row) => row.agent },
  {
    id: 'severity',
    header: 'Severity',
    cell: (row) => (
      <Badge variant="outline" className="rounded-sm">
        {row.severity}
      </Badge>
    ),
  },
  { id: 'action', header: 'Action', cell: (row) => row.action },
  { id: 'enabled', header: 'Enabled', cell: (row) => (row.enabled ? 'Yes' : 'No') },
];

export function PoliciesPageContent({ data }: { data: PoliciesPageData }) {
  return (
    <div className="grid gap-4 px-4 lg:px-6">
      <div className="flex flex-col gap-3 md:flex-row md:items-end md:justify-between">
        <div>
          <p className="text-sm text-muted-foreground">{data.activeWorkspace.name}</p>
          <h2 className="text-2xl font-semibold">Policies</h2>
        </div>
        <div className="flex flex-col gap-2 sm:flex-row">
          <PolicyCreateDialog agents={data.agents} workspaceSlug={data.activeWorkspace.slug}>
            <IconPlus />
            New policy
          </PolicyCreateDialog>
        </div>
      </div>

      <Card>
        <CardHeader>
          <CardDescription>Workspace-authored guardrails</CardDescription>
          <CardTitle>Policies</CardTitle>
        </CardHeader>
        <CardContent>
          <DataTable
            columns={policyColumns}
            rows={data.policies}
            getRowKey={(policy) => policy.id}
            empty="No policies authored yet."
          />
        </CardContent>
      </Card>

    </div>
  );
}
