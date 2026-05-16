'use client';

import { IconPlus } from '@tabler/icons-react';

import { Badge } from '@/components/ui/badge';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import { PolicyCreateDialog } from '@/components/workspace/PolicyCreateDialog';
import type { AgentRow, DashboardShellData, PolicyRow } from '@/lib/server/dashboard-data';

type PoliciesPageData = DashboardShellData & { agents: AgentRow[]; policies: PolicyRow[] };

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
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Policy</TableHead>
                <TableHead>Description</TableHead>
                <TableHead>Agent</TableHead>
                <TableHead>Severity</TableHead>
                <TableHead>Action</TableHead>
                <TableHead>Enabled</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {data.policies.map((policy) => (
                <TableRow key={policy.id}>
                  <TableCell className="font-mono text-xs">{policy.id}</TableCell>
                  <TableCell className="text-muted-foreground">{policy.description}</TableCell>
                  <TableCell>{policy.agent}</TableCell>
                  <TableCell>
                    <Badge variant="outline" className="rounded-sm">
                      {policy.severity}
                    </Badge>
                  </TableCell>
                  <TableCell>{policy.action}</TableCell>
                  <TableCell>{policy.enabled ? 'Yes' : 'No'}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </CardContent>
      </Card>

    </div>
  );
}
