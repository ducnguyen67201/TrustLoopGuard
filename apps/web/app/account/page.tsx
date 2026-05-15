import { AppLayout } from '@/components/AppLayout';
import { AccountPageContent } from '@/components/workspace/ManagementPages';
import { auth } from '@/auth';
import { getDashboardShell } from '@/lib/server/dashboard-data';

import { ChangePasswordCard } from './change-password-card';

export default async function AccountPage() {
  const [data, session] = await Promise.all([getDashboardShell(), auth()]);

  const user = session?.user as
    | { loginMethod?: string; username?: string; name?: string | null }
    | undefined;
  const showChangePassword = user?.loginMethod === 'credentials';
  const username = user?.username ?? user?.name ?? '';

  return (
    <AppLayout title="Account">
      <AccountPageContent data={data} />
      {showChangePassword && username ? (
        <div className="px-4 lg:px-6">
          <ChangePasswordCard username={username} />
        </div>
      ) : null}
    </AppLayout>
  );
}
