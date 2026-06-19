'use client';

import { IconBook2 } from '@tabler/icons-react';

import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from '@/components/ui/dialog';
import { KnowledgeSourceForm } from '@/components/workspace/KnowledgeSourceForm';

export function KnowledgeSourceCreateDialog({ workspaceSlug }: { workspaceSlug: string }) {
  const knowledgeHref = `/knowledge-sources?workspace=${workspaceSlug}`;

  return (
    <Dialog>
      <DialogTrigger asChild>
        <Button>
          <IconBook2 />
          Add source
        </Button>
      </DialogTrigger>
      <DialogContent className="max-h-[90vh] max-w-xl overflow-y-auto">
        <DialogHeader>
          <DialogTitle>Add a knowledge source</DialogTitle>
          <DialogDescription>
            Add content you trust — a file, a web link, or pasted text — so the guardrail can check
            answers against it.
          </DialogDescription>
        </DialogHeader>

        <KnowledgeSourceForm
          workspaceSlug={workspaceSlug}
          cancelHref={knowledgeHref}
          variant="dialog"
          cancelSlot={
            <DialogClose asChild>
              <Button variant="outline" type="button">
                Cancel
              </Button>
            </DialogClose>
          }
        />
      </DialogContent>
    </Dialog>
  );
}
