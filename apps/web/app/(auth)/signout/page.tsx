import Link from 'next/link';
import { signOut } from '@/auth';
import { BrandLogo } from '@/components/brand-logo';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';

export const dynamic = 'force-dynamic';

export default async function SignOutPage() {
  return (
    <main className="flex min-h-screen items-center justify-center bg-background px-4 py-10 text-foreground">
      <div className="grid w-full max-w-md gap-4">
        <div className="grid gap-2">
          <div className="flex items-center gap-2 text-sm text-muted-foreground">
            <BrandLogo className="size-7" priority />
            <span>TrustLoopGuard</span>
          </div>
          <h1 className="text-2xl font-semibold">Sign out</h1>
        </div>
        <Card>
          <CardHeader>
            <CardTitle>Leave this session?</CardTitle>
            <CardDescription>
              You can sign back in with your workspace identity when you need access again.
            </CardDescription>
          </CardHeader>
          <CardContent className="grid gap-3">
            <form
              action={async () => {
                'use server';

                await signOut({ redirectTo: '/signin' });
              }}
            >
              <Button type="submit" className="w-full">
                Sign out
              </Button>
            </form>
            <Button asChild variant="outline" className="w-full">
              <Link href="/">Cancel</Link>
            </Button>
          </CardContent>
        </Card>
      </div>
    </main>
  );
}
