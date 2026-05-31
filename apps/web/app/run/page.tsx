import { redirect } from 'next/navigation';

export default async function RunRedirectPage({
  searchParams,
}: {
  searchParams: Promise<{ id?: string | string[]; workspace?: string | string[] }>;
}) {
  const resolvedSearchParams = await searchParams;
  const id = firstParam(resolvedSearchParams.id);
  if (!id) {
    redirect('/runs');
  }

  const workspace = firstParam(resolvedSearchParams.workspace);
  const workspaceQuery = workspace ? `?workspace=${encodeURIComponent(workspace)}` : '';
  redirect(`/runs/${encodeURIComponent(id)}${workspaceQuery}`);
}

function firstParam(value: string | string[] | undefined): string | null {
  if (Array.isArray(value)) return value[0] ?? null;
  return value ?? null;
}
