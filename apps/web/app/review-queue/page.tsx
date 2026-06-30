import { AppLayout } from '@/components/AppLayout';
import { ReviewQueueContent } from '@/components/workspace/ReviewQueueContent';
import { readParam } from '@/lib/search-params';

export default async function ReviewQueuePage({
  searchParams,
}: {
  searchParams: Promise<{ workspace?: string | string[]; environment?: string | string[] }>;
}) {
  const params = await searchParams;
  const workspaceSlug = readParam(params.workspace);
  const environmentId = readParam(params.environment);

  return (
    <AppLayout title="Review queue" workspaceSlug={workspaceSlug} environmentId={environmentId}>
      <ReviewQueueContent workspaceSlug={workspaceSlug ?? ''} />
    </AppLayout>
  );
}
