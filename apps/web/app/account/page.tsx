import { AppLayout } from '@/components/AppLayout';
import { AccountPageContent } from '@/components/workspace/ManagementPages';
import { getDashboardShell } from '@/lib/server/dashboard-data';

export default async function AccountPage() {
  const data = await getDashboardShell();

  return (
    <AppLayout title="Account">
      <AccountPageContent data={data} />
    </AppLayout>
  );
}
