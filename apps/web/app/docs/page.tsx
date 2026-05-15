import Link from 'next/link';
import { IconExternalLink } from '@tabler/icons-react';

import { AppLayout } from '@/components/AppLayout';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';

export default function DocsPage() {
  return (
    <AppLayout title="Docs">
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
    </AppLayout>
  );
}
