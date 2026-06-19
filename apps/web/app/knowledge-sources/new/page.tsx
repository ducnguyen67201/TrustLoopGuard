import { IconArrowLeft } from '@tabler/icons-react';

import { AppLayout } from '@/components/AppLayout';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { PageHeader } from '@/components/ui/page-header';
import { KnowledgeSourceForm } from '@/components/workspace/KnowledgeSourceForm';
import { readWorkspaceSlug } from '@/lib/search-params';
import { getDashboardShell } from '@/lib/server/dashboard-data';

export default async function NewKnowledgeSourcePage({
  searchParams,
}: {
  searchParams: Promise<{ workspace?: string | string[] }>;
}) {
  const workspaceSlug = readWorkspaceSlug(await searchParams);
  const data = await getDashboardShell(workspaceSlug);
  const knowledgeHref = `/knowledge-sources?workspace=${data.activeWorkspace.slug}`;

  return (
    <AppLayout
      title="Add source"
      workspaceSlug={workspaceSlug}
      breadcrumbs={[{ label: 'Knowledge', href: knowledgeHref }, { label: 'Add source' }]}
    >
      <div className="grid gap-6 px-4 lg:px-6">
        <PageHeader
          eyebrow={data.activeWorkspace.name}
          title="Add knowledge source"
          description="Give the guardrail engine approved context to ground its decisions on — a hosted URL, an uploaded document, or an inline note."
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
