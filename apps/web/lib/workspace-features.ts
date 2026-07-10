export type WorkspaceFeature = 'attacks' | 'knowledgeBase';

interface WorkspaceFeatureFlags {
  isAttacksEnabled: boolean;
  isKnowledgeBaseEnabled: boolean;
}

export function isWorkspaceFeatureEnabled(
  workspace: WorkspaceFeatureFlags,
  feature: WorkspaceFeature,
): boolean {
  return feature === 'attacks'
    ? workspace.isAttacksEnabled
    : workspace.isKnowledgeBaseEnabled;
}
