'use client';

import { IconMailForward } from '@tabler/icons-react';
import { useRouter, useSearchParams } from 'next/navigation';
import { useState } from 'react';
import { toast } from 'sonner';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { DataTable, type DataTableColumn } from '@/components/ui/data-table';
import { EmptyState } from '@/components/ui/empty-state';
import { InviteMemberDialog } from '@/components/workspace/InviteMemberDialog';
import type { TeamInviteRow } from '@/lib/server/dashboard-data';

export function PendingInvitesTable({ invites }: { invites: TeamInviteRow[] }) {
  const router = useRouter();
  const searchParams = useSearchParams();
  const workspace = searchParams.get('workspace') ?? '';
  const [busyId, setBusyId] = useState<string | null>(null);

  async function revoke(invite: TeamInviteRow) {
    if (busyId !== null) return;
    setBusyId(invite.id);
    const queryString = workspace
      ? `?workspace=${encodeURIComponent(workspace)}`
      : '';
    try {
      const res = await fetch(
        `/api/team/invites/${encodeURIComponent(invite.id)}${queryString}`,
        { method: 'DELETE' },
      );
      if (!res.ok) {
        const text = await res.text();
        toast.error(text || `revoke failed (${res.status})`);
        return;
      }
      toast.success('Invite revoked');
      router.refresh();
    } catch (err) {
      const message = err instanceof Error ? err.message : 'revoke failed';
      toast.error(message);
    } finally {
      setBusyId(null);
    }
  }

  if (invites.length === 0) {
    return (
      <EmptyState
        icon={<IconMailForward />}
        title="No one is waiting to join"
        description="Invite a teammate by email. They'll appear here until they sign up, then join this workspace automatically."
        action={<InviteMemberDialog />}
      />
    );
  }

  const columns: DataTableColumn<TeamInviteRow>[] = [
    {
      id: 'email',
      header: 'Email',
      cell: (invite) => <span className="font-mono">{invite.email}</span>,
    },
    {
      id: 'role',
      header: 'Role',
      cell: (invite) => (
        <Badge variant="outline" className="rounded-sm capitalize">
          {invite.role}
        </Badge>
      ),
    },
    {
      id: 'status',
      header: 'Status',
      cell: (invite) => <span className="text-muted-foreground capitalize">{invite.status}</span>,
    },
    {
      id: 'invitedAt',
      header: 'Invited',
      cell: (invite) => <span className="text-muted-foreground">{invite.invitedAt}</span>,
    },
    {
      id: 'expiresAt',
      header: 'Expires',
      cell: (invite) => <span className="text-muted-foreground">{invite.expiresAt}</span>,
    },
    {
      id: 'actions',
      header: 'Actions',
      align: 'right',
      cell: (invite) => (
        <Button
          type="button"
          variant="ghost"
          size="sm"
          disabled={busyId === invite.id}
          onClick={() => revoke(invite)}
        >
          {busyId === invite.id ? 'Revoking…' : 'Revoke'}
        </Button>
      ),
    },
  ];

  return (
    <DataTable
      columns={columns}
      rows={invites}
      getRowKey={(invite) => invite.id}
      caption="Pending workspace invites"
      empty="No one is waiting to join."
    />
  );
}
