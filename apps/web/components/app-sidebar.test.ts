import { describe, expect, it } from 'vitest';

import { getVisibleNavGroups } from './app-sidebar';

const workspace = {
  isAttacksEnabled: false,
  isKnowledgeBaseEnabled: false,
  isMcpGatewayEnabled: false,
};

describe('getVisibleNavGroups', () => {
  it('omits unreleased workspace features', () => {
    const titles = getVisibleNavGroups(workspace, (url) => url).flatMap((group) =>
      group.items.map((item) => item.title),
    );

    expect(titles).not.toContain('Attacks (Beta)');
    expect(titles).not.toContain('Knowledge sources (Beta)');
    expect(titles).not.toContain('MCP Access');
  });

  it('shows independently enabled workspace features', () => {
    const titles = getVisibleNavGroups(
      { ...workspace, isAttacksEnabled: true },
      (url) => url,
    ).flatMap((group) => group.items.map((item) => item.title));

    expect(titles).toContain('Attacks (Beta)');
    expect(titles).not.toContain('Knowledge sources (Beta)');
  });

  it('labels enabled knowledge sources as beta', () => {
    const titles = getVisibleNavGroups(
      { ...workspace, isKnowledgeBaseEnabled: true },
      (url) => url,
    ).flatMap((group) => group.items.map((item) => item.title));

    expect(titles).toContain('Knowledge sources (Beta)');
  });

  it('shows MCP Access only for enabled workspaces', () => {
    const titles = getVisibleNavGroups(
      { ...workspace, isMcpGatewayEnabled: true },
      (url) => url,
    ).flatMap((group) => group.items.map((item) => item.title));

    expect(titles).toContain('MCP Access');
  });
});
