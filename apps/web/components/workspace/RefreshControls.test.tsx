import { renderToString } from 'react-dom/server';
import { describe, expect, it, vi } from 'vitest';

import { RefreshControls } from './RefreshControls';

describe('RefreshControls', () => {
  it('does not server-render a variable last-sync timestamp', () => {
    const html = renderToString(
      <RefreshControls
        mode="live"
        onModeChange={vi.fn()}
        onRefresh={vi.fn()}
        isRefreshing={false}
        lastSync={new Date('2026-06-25T18:28:11.799Z')}
      />,
    );

    expect(html).toContain('just now');
    expect(html).not.toContain('2026-06-25T18:28:11.799Z');
  });
});
