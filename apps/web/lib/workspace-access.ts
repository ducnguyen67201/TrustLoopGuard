export type WorkspaceMembership = {
  id: string;
  slug: string;
};

export function selectAuthorizedWorkspaceId(
  memberships: WorkspaceMembership[],
  requestedWorkspace: string | null | undefined,
): string | null {
  if (memberships.length === 0) return null;

  const requested = requestedWorkspace?.trim();
  if (requested === undefined || requested === '') {
    return memberships[0]!.id;
  }

  return (
    memberships.find((workspace) => workspace.slug === requested || workspace.id === requested)
      ?.id ?? null
  );
}
