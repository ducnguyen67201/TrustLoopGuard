import { describe, expect, it } from 'vitest';

import { getVisibleNavGroups } from './app-sidebar';

const workspace = {
  isAttacksEnabled: false,
  isKnowledgeBaseEnabled: false,
};

describe('getVisibleNavGroups', () => {
  it('omits unreleased workspace features', () => {
    const titles = getVisibleNavGroups(workspace, (url) => url).flatMap((group) =>
      group.items.map((item) => item.title),
    );

    expect(titles).not.toContain('Attacks');
    expect(titles).not.toContain('Knowledge sources');
  });

  it('shows independently enabled workspace features', () => {
    const titles = getVisibleNavGroups(
      { ...workspace, isAttacksEnabled: true },
      (url) => url,
    ).flatMap((group) => group.items.map((item) => item.title));

    expect(titles).toContain('Attacks');
    expect(titles).not.toContain('Knowledge sources');
  });
});
