'use client';

import { useRouter, useSearchParams } from 'next/navigation';
import { useState } from 'react';
import { toast } from 'sonner';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import type { TeamInviteRow } from '@/lib/server/dashboard-data';

export function PendingInvitesTable({ invites }: { invites: TeamInviteRow[] }) {
  const router = useRouter();
  const searchParams = useSearchParams();
  const workspace = searchParams.get('workspace') ?? '';
  const [busyId, setBusyId] = useState<string | null>(null);

  if (invites.length === 0) {
    return (
      <p className="text-sm text-muted-foreground">
        No pending invites. Use “Invite member” to add a teammate.
      </p>
    );
  }

  async function copy(invite: TeamInviteRow) {
    const link = `${window.location.origin}${invite.acceptPath}`;
    try {
      await navigator.clipboard.writeText(link);
      toast.success('Accept link copied');
    } catch {
      toast.error('Could not access clipboard — copy manually');
    }
  }

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

  return (
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead>Email</TableHead>
          <TableHead>Role</TableHead>
          <TableHead>Status</TableHead>
          <TableHead>Invited</TableHead>
          <TableHead>Expires</TableHead>
          <TableHead className="text-right">Actions</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {invites.map((invite) => (
          <TableRow key={invite.id}>
            <TableCell className="font-medium">{invite.email}</TableCell>
            <TableCell>
              <Badge variant="outline" className="rounded-sm">
                {invite.role}
              </Badge>
            </TableCell>
            <TableCell className="text-muted-foreground">{invite.status}</TableCell>
            <TableCell className="text-muted-foreground">{invite.invitedAt}</TableCell>
            <TableCell className="text-muted-foreground">{invite.expiresAt}</TableCell>
            <TableCell className="space-x-2 text-right">
              <Button
                type="button"
                variant="outline"
                size="sm"
                onClick={() => copy(invite)}
              >
                Copy link
              </Button>
              <Button
                type="button"
                variant="ghost"
                size="sm"
                disabled={busyId === invite.id}
                onClick={() => revoke(invite)}
              >
                {busyId === invite.id ? 'Revoking…' : 'Revoke'}
              </Button>
            </TableCell>
          </TableRow>
        ))}
      </TableBody>
    </Table>
  );
}
