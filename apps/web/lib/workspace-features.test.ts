import { describe, expect, it } from 'vitest';

import { isWorkspaceFeatureEnabled } from './workspace-features';

describe('isWorkspaceFeatureEnabled', () => {
  const disabledWorkspace = {
    isAttacksEnabled: false,
    isKnowledgeBaseEnabled: false,
    isMcpGatewayEnabled: false,
  };

  it('keeps attacks unavailable by default', () => {
    expect(isWorkspaceFeatureEnabled(disabledWorkspace, 'attacks')).toBe(false);
  });

  it('keeps knowledge sources unavailable by default', () => {
    expect(isWorkspaceFeatureEnabled(disabledWorkspace, 'knowledgeBase')).toBe(false);
  });

  it('keeps MCP access unavailable by default', () => {
    expect(isWorkspaceFeatureEnabled(disabledWorkspace, 'mcpAccess')).toBe(false);
  });

  it('allows each feature to be rolled out independently', () => {
    expect(
      isWorkspaceFeatureEnabled(
        { ...disabledWorkspace, isAttacksEnabled: true },
        'attacks',
      ),
    ).toBe(true);
    expect(
      isWorkspaceFeatureEnabled(
        { ...disabledWorkspace, isKnowledgeBaseEnabled: true },
        'knowledgeBase',
      ),
    ).toBe(true);
    expect(
      isWorkspaceFeatureEnabled(
        { ...disabledWorkspace, isMcpGatewayEnabled: true },
        'mcpAccess',
      ),
    ).toBe(true);
  });
});
