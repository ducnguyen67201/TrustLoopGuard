import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';

import { SwitchyardMap } from './SwitchyardMap';

describe('SwitchyardMap', () => {
  it('renders real servers before assignment and policy junctions', () => {
    render(<SwitchyardMap connections={[{ id: 'connection', display_name: 'GitHub', server_slug: 'github', endpoint_url: 'https://mcp.example/mcp', auth_kind: 'none', credential_status: 'not_required', enabled: true, last_sync_status: 'succeeded', tool_count: 2, created_at: '', updated_at: '' }]} />);
    expect(screen.getByText('GitHub')).toBeInTheDocument();
    expect(screen.getByText('Tool access')).toBeInTheDocument();
    expect(screen.getByText('Runtime policy')).toBeInTheDocument();
  });
});
