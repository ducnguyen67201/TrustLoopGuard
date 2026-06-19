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
        toast.error(text || `Couldn't cancel that invite — please try again (${res.status})`);
        return;
      }
      toast.success('Invite cancelled');
      router.refresh();
    } catch (err) {
      const message = err instanceof Error ? err.message : "Couldn't cancel that invite — please try again";
      toast.error(message);
    } finally {
      setBusyId(null);
    }
  }

  if (invites.length === 0) {
    return (
      <EmptyState
        icon={<IconMailForward />}
        title="No pending invites"
        description="A pending invite is someone you've invited who hasn't joined yet. Invite a teammate by email — they'll show up here until they sign up, then join this workspace automatically."
        action={<InviteMemberDialog />}
      />
    );
  }

  const columns: DataTableColumn<TeamInviteRow>[] = [
    {
      id: 'email',
      header: 'Invited email',
      cell: (invite) => <span className="font-mono">{invite.email}</span>,
    },
    {
      id: 'role',
      header: 'Will join as',
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
      header: 'Sent',
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
      cell: (invite) => {
        const isBusy = busyId === invite.id;
        return (
          <Button
            type="button"
            variant="ghost"
            size="sm"
            disabled={isBusy}
            aria-busy={isBusy}
            onClick={() => revoke(invite)}
          >
            {isBusy ? 'Cancelling…' : 'Cancel invite'}
          </Button>
        );
      },
    },
  ];

  return (
    <DataTable
      columns={columns}
      rows={invites}
      getRowKey={(invite) => invite.id}
      caption="People you've invited who haven't joined yet"
      empty="No one is waiting to join."
    />
  );
}
