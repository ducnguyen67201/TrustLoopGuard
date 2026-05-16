'use client';

import { IconBook2 } from '@tabler/icons-react';

import { createKnowledgeSource } from '@/app/knowledge-sources/actions';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Textarea } from '@/components/ui/textarea';

export function KnowledgeSourceCreateDialog({ workspaceSlug }: { workspaceSlug: string }) {
  return (
    <Dialog>
      <DialogTrigger asChild>
        <Button>
          <IconBook2 />
          Add source
        </Button>
      </DialogTrigger>
      <DialogContent className="max-h-[90vh] max-w-3xl overflow-y-auto">
        <DialogHeader>
          <DialogTitle>Add knowledge source</DialogTitle>
          <DialogDescription>
            Add a workspace-owned URL, note, or uploaded file for guardrail context.
          </DialogDescription>
        </DialogHeader>

        <form action={createKnowledgeSource} className="grid gap-5" encType="multipart/form-data">
          <input type="hidden" name="workspaceSlug" value={workspaceSlug} />

          <Field label="Title" htmlFor="title">
            <Input id="title" name="title" placeholder="Refund policy" required />
          </Field>

          <div className="grid gap-4 md:grid-cols-2">
            <Field label="Kind" htmlFor="kind">
              <Select name="kind" defaultValue="url" required>
                <SelectTrigger id="kind" className="w-full">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="url">URL</SelectItem>
                  <SelectItem value="file">File</SelectItem>
                  <SelectItem value="note">Note</SelectItem>
                </SelectContent>
              </Select>
            </Field>

            <Field label="Location" htmlFor="location">
              <Input id="location" name="location" placeholder="https://example.com/help/refunds" />
            </Field>
          </div>

          <Field label="File upload" htmlFor="file">
            <Input id="file" name="file" type="file" />
            <p className="text-xs text-muted-foreground">
              Required for File sources. Maximum size is 10 MB.
            </p>
          </Field>

          <Field label="Notes" htmlFor="notes">
            <Textarea
              id="notes"
              name="notes"
              placeholder="What should the team know about this source?"
            />
          </Field>

          <div className="flex justify-end gap-2">
            <Button type="submit">Add source</Button>
          </div>
        </form>
      </DialogContent>
    </Dialog>
  );
}

function Field({
  label,
  htmlFor,
  children,
}: {
  label: string;
  htmlFor: string;
  children: React.ReactNode;
}) {
  return (
    <div className="grid gap-2">
      <Label htmlFor={htmlFor}>{label}</Label>
      {children}
    </div>
  );
}
