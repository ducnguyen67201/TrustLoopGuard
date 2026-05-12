import { auth } from '@/auth';
import { AppShell } from '@/components/AppShell';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';

export default async function DashboardPage() {
  const session = await auth();

  return (
    <AppShell title="Dashboard" className="max-w-4xl">
      <Card>
        <CardHeader>
          <CardTitle>Session</CardTitle>
          <CardDescription>Current authenticated user context.</CardDescription>
        </CardHeader>
        <CardContent className="font-mono text-sm text-muted-foreground">
          Signed in as {session?.user?.email ?? 'unknown'}.
        </CardContent>
      </Card>
    </AppShell>
  );
}
