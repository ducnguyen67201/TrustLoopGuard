import { IconArrowLeft } from '@tabler/icons-react';
import { notFound } from 'next/navigation';

import { AppLayout } from '@/components/AppLayout';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { PageHeader } from '@/components/ui/page-header';
import { KnowledgeSourceForm } from '@/components/workspace/KnowledgeSourceForm';
import { readWorkspaceSlug } from '@/lib/search-params';
import { getDashboardShell } from '@/lib/server/dashboard-data';
import { isWorkspaceFeatureEnabled } from '@/lib/workspace-features';

export default async function NewKnowledgeSourcePage({
  searchParams,
}: {
  searchParams: Promise<{ workspace?: string | string[] }>;
}) {
  const workspaceSlug = readWorkspaceSlug(await searchParams);
  const data = await getDashboardShell(workspaceSlug);
  if (!isWorkspaceFeatureEnabled(data.activeWorkspace, 'knowledgeBase')) notFound();
  const role = data.activeWorkspace.role.toLowerCase();
  if (role !== 'owner' && role !== 'admin') notFound();
  const knowledgeHref = `/knowledge-sources?workspace=${data.activeWorkspace.slug}`;

  return (
    <AppLayout
      title="Add source"
      workspaceSlug={workspaceSlug}
      shell={data}
      breadcrumbs={[{ label: 'Knowledge', href: knowledgeHref }, { label: 'Add source' }]}
    >
      <div className="grid gap-6 px-4 lg:px-6">
        <PageHeader
          eyebrow={data.activeWorkspace.name}
          title="Add a knowledge source"
          description="Add content you trust — a file, a web link, or pasted text — so the guardrail can check answers against your own material."
          actions={
            <Button variant="outline" asChild>
              <a href={knowledgeHref}>
                <IconArrowLeft />
                Back to knowledge
              </a>
            </Button>
          }
        />

        <Card className="max-w-2xl">
          <CardContent>
            <KnowledgeSourceForm
              workspaceSlug={data.activeWorkspace.slug}
              cancelHref={knowledgeHref}
              variant="page"
            />
          </CardContent>
        </Card>
      </div>
    </AppLayout>
  );
}
