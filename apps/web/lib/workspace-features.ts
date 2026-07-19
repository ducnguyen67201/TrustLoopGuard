export type WorkspaceFeature = 'attacks' | 'knowledgeBase' | 'mcpAccess';

interface WorkspaceFeatureFlags {
  isAttacksEnabled: boolean;
  isKnowledgeBaseEnabled: boolean;
  isMcpGatewayEnabled: boolean;
}

export function isWorkspaceFeatureEnabled(
  workspace: WorkspaceFeatureFlags,
  feature: WorkspaceFeature,
): boolean {
  if (feature === 'attacks') return workspace.isAttacksEnabled;
  if (feature === 'knowledgeBase') return workspace.isKnowledgeBaseEnabled;
  return workspace.isMcpGatewayEnabled;
}
