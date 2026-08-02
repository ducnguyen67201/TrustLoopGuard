import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import type { PolicyDocument } from '@featherlane-ai/sdk';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { PolicyEditorDialog } from './PolicyEditorDialog';

const getPolicy = vi.fn<(id: string) => Promise<PolicyDocument>>();
const upsertPolicy = vi.fn<(yaml: string) => Promise<{ id: string }>>();
const validatePolicy = vi.fn();
const generatePolicyDraft = vi.fn();

vi.mock('@/lib/policies', () => ({
  getPolicy: (id: string) => getPolicy(id),
  upsertPolicy: (yaml: string) => upsertPolicy(yaml),
  validatePolicy: (yaml: string) => validatePolicy(yaml),
  generatePolicyDraft: (prompt: string) => generatePolicyDraft(prompt),
}));

vi.mock('sonner', () => ({
  toast: { success: vi.fn(), error: vi.fn() },
}));

// A rule the guided builder CAN round-trip (match.literal). The real action and
// match live only in source_yaml — the PolicyDocument type carries no `action`
// field — so the form must parse them out of the YAML, not hardcode defaults.
const ROUNDTRIP_YAML = [
  'id: refund-rule',
  'description: Refund rule',
  'match:',
  '  literal: guaranteed refund',
  'action: transform',
  'severity: high',
  '',
].join('\n');

// A rule the guided builder CANNOT round-trip: match.contains is neither
// `literal` nor `regex`, so yamlToDraft rejects it.
const NON_ROUNDTRIP_YAML = [
  'id: legacy-refund',
  'description: Legacy refund rule',
  'match:',
  '  contains: guaranteed refund',
  'action: require_approval',
  'severity: high',
  '',
].join('\n');

function makeDocument(overrides: Partial<PolicyDocument> = {}): PolicyDocument {
  return {
    id: 'legacy-refund',
    family: 'content',
    description: 'Legacy refund rule',
    severity: 'high',
    enabled: true,
    source_yaml: NON_ROUNDTRIP_YAML,
    ...overrides,
  };
}

afterEach(() => {
  vi.clearAllMocks();
});

describe('PolicyEditorDialog edit mode', () => {
  it('loads the rule’s real action from its YAML instead of hardcoding block', async () => {
    getPolicy.mockResolvedValue(
      makeDocument({ id: 'refund-rule', description: 'Refund rule', source_yaml: ROUNDTRIP_YAML }),
    );

    render(
      <PolicyEditorDialog
        open
        mode={{ kind: 'edit', policyId: 'refund-rule' }}
        onOpenChange={vi.fn()}
        onSaved={vi.fn()}
      />,
    );

    // The action dropdown reflects the real transform effect instead of hardcoding denial.
    await waitFor(() => {
      const actionTrigger = document.getElementById('action');
      expect(actionTrigger).not.toBeNull();
      expect(actionTrigger).toHaveTextContent('Use a safe transformed value');
    });
  });

  it('never claims a literal/empty match it cannot read (unreadable match)', async () => {
    getPolicy.mockResolvedValue(makeDocument());

    render(
      <PolicyEditorDialog
        open
        mode={{ kind: 'edit', policyId: 'legacy-refund' }}
        onOpenChange={vi.fn()}
        onSaved={vi.fn()}
      />,
    );

    await screen.findByText(/guided editor can't show/i);
    // It must NOT print the false "contains your chosen words" / empty-match summary.
    expect(screen.queryByText(/contains your chosen words/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/contains “”/)).not.toBeInTheDocument();
  });

  it('saving an unreadable-match rule preserves the original source_yaml verbatim', async () => {
    getPolicy.mockResolvedValue(makeDocument());
    upsertPolicy.mockResolvedValue({ id: 'legacy-refund' });
    const onSaved = vi.fn();

    render(
      <PolicyEditorDialog
        open
        mode={{ kind: 'edit', policyId: 'legacy-refund' }}
        onOpenChange={vi.fn()}
        onSaved={onSaved}
      />,
    );

    const saveButton = await screen.findByRole('button', { name: /save changes/i });
    await userEvent.click(saveButton);

    await waitFor(() => {
      expect(upsertPolicy).toHaveBeenCalledTimes(1);
    });
    // The rule is saved exactly as it was loaded — its match/logic is untouched.
    expect(upsertPolicy).toHaveBeenCalledWith(NON_ROUNDTRIP_YAML);
    expect(onSaved).toHaveBeenCalledWith('legacy-refund');
  });
});
