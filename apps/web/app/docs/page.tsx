import type { ReactNode } from 'react';
import Link from 'next/link';
import { IconExternalLink } from '@tabler/icons-react';

import { AppLayout } from '@/components/AppLayout';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { getOptionalDashboardShell } from '@/lib/server/dashboard-data';

interface DocsPageProps {
  searchParams?: Promise<{ workspace?: string }>;
}

export default async function DocsPage({ searchParams }: DocsPageProps) {
  const { workspace } = (await searchParams) ?? {};
  const shell = await getOptionalDashboardShell(workspace);
  const content = <DocsContent />;

  if (shell) {
    return (
      <AppLayout title="Docs" shell={shell}>
        {content}
      </AppLayout>
    );
  }

  return <PublicDocsPage>{content}</PublicDocsPage>;
}

function PublicDocsPage({ children }: { children: ReactNode }) {
  return (
    <main className="min-h-svh bg-background py-8">
      <div className="mx-auto flex w-full max-w-4xl flex-col gap-6 px-4 lg:px-6">
        <Link href="/" className="w-fit text-sm font-medium text-muted-foreground hover:text-foreground">
          TrustLoopGuard
        </Link>
        {children}
      </div>
    </main>
  );
}

function DocsContent() {
  return (
    <div className="px-4 lg:px-6">
      <Card>
        <CardHeader>
          <CardDescription>Product documentation</CardDescription>
          <CardTitle>TrustLoopGuard docs</CardTitle>
        </CardHeader>
        <CardContent className="grid gap-4">
          <p className="max-w-2xl text-sm text-muted-foreground">
            Product docs live in the docs app. This dashboard entry keeps documentation one click
            away from every workspace.
          </p>
          <Button asChild className="w-fit" variant="outline">
            <Link href="http://localhost:3001">
              Open docs
              <IconExternalLink />
            </Link>
          </Button>
        </CardContent>
      </Card>
    </div>
  );
}
