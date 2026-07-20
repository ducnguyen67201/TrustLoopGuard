'use client';

import { IconTrash } from '@tabler/icons-react';
import { useRouter } from 'next/navigation';
import { useId, useRef, useState } from 'react';
import { toast } from 'sonner';

import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from '@/components/ui/alert-dialog';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { http } from '@/lib/http';

interface DeleteWorkspaceDialogProps {
  workspaceId: string;
  workspaceName: string;
  workspaceRole: string;
  isActive: boolean;
  fallbackWorkspaceSlug: string | null;
}

export function DeleteWorkspaceDialog({
  workspaceId,
  workspaceName,
  workspaceRole,
  isActive,
  fallbackWorkspaceSlug,
}: DeleteWorkspaceDialogProps) {
  const router = useRouter();
  const confirmationId = useId();
  const deletingRef = useRef(false);
  const [open, setOpen] = useState(false);
  const [confirmation, setConfirmation] = useState('');
  const [deleting, setDeleting] = useState(false);

  if (workspaceRole.toLowerCase() !== 'owner') return null;

  function handleOpenChange(nextOpen: boolean) {
    if (deletingRef.current) return;
    setOpen(nextOpen);
    if (!nextOpen) setConfirmation('');
  }

  async function deleteWorkspace() {
    if (deletingRef.current || confirmation !== workspaceName) return;
    deletingRef.current = true;
    setDeleting(true);

    try {
      await http.withoutWorkspace.delete(`/api/me/workspaces/${encodeURIComponent(workspaceId)}`);
      setOpen(false);
      setConfirmation('');
      toast.success(`Workspace “${workspaceName}” deleted`);

      if (!isActive) {
        router.refresh();
      } else if (fallbackWorkspaceSlug !== null) {
        router.replace(`/workspaces?workspace=${encodeURIComponent(fallbackWorkspaceSlug)}`);
        router.refresh();
      } else {
        router.replace('/onboarding/workspace');
      }
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Could not delete workspace');
    } finally {
      deletingRef.current = false;
      setDeleting(false);
    }
  }

  return (
    <AlertDialog open={open} onOpenChange={handleOpenChange}>
      <AlertDialogTrigger asChild>
        <Button
          type="button"
          variant="ghost"
          size="sm"
          aria-label={`Delete ${workspaceName}`}
          className="text-destructive hover:text-destructive"
        >
          <IconTrash />
          Delete
        </Button>
      </AlertDialogTrigger>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>Delete “{workspaceName}”?</AlertDialogTitle>
          <AlertDialogDescription>
            This removes the workspace from the dashboard for everyone. This action cannot be undone
            here.
          </AlertDialogDescription>
        </AlertDialogHeader>
        <ul className="grid list-disc gap-1 pl-5 text-sm text-muted-foreground">
          <li>All members will lose access.</li>
          <li>Pending invitations will be revoked.</li>
          <li>Active runtime API keys will stop working.</li>
          <li>Historical guardrail records will be retained.</li>
        </ul>
        <div className="grid gap-2">
          <Label htmlFor={confirmationId}>Type “{workspaceName}” to confirm</Label>
          <Input
            id={confirmationId}
            value={confirmation}
            onChange={(event) => setConfirmation(event.target.value)}
            disabled={deleting}
            autoComplete="off"
          />
        </div>
        <AlertDialogFooter>
          <AlertDialogCancel disabled={deleting}>Keep workspace</AlertDialogCancel>
          <AlertDialogAction
            variant="destructive"
            disabled={deleting || confirmation !== workspaceName}
            onClick={(event) => {
              event.preventDefault();
              void deleteWorkspace();
            }}
          >
            {deleting ? 'Deleting…' : 'Delete workspace'}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
