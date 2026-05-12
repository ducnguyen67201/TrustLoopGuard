import { AppShell } from '@/components/AppShell';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';

export default function SignInPage() {
  return (
    <AppShell title="Sign in" className="max-w-xl">
      <Card>
        <CardHeader>
          <CardTitle>Authentication unavailable</CardTitle>
          <CardDescription>No sign-in methods are configured for this deployment.</CardDescription>
        </CardHeader>
        <CardContent className="text-sm text-muted-foreground">
          Set <code className="font-mono text-foreground">AUTH_ALLOW_SIGNUP</code> to enable email
          and password, or <code className="font-mono text-foreground">AUTH_GOOGLE_ID</code> and{' '}
          <code className="font-mono text-foreground">AUTH_GOOGLE_SECRET</code> to enable Google.
        </CardContent>
      </Card>
    </AppShell>
  );
}
