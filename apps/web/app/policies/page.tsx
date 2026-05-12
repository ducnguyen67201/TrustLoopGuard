import { AppShell } from '@/components/AppShell';
import { PolicyManager } from '@/components/policies/PolicyManager';
import { getServerUrl } from '@/lib/server-url';

export default function PoliciesPage() {
  const serverUrl = getServerUrl();

  return (
    <AppShell
      title="Policy Manager"
      description="Author YAML policies, validate them, and manage their enabled state through the tl-server policy API."
      active="policies"
      footer={
        <>
          <span>{serverUrl}/v1/policies</span>
          <span>YAML + JSON API</span>
        </>
      }
    >
      <PolicyManager />
    </AppShell>
  );
}
