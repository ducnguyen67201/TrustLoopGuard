import { DocsLayout } from 'fumadocs-ui/layouts/docs';
import type { ReactNode } from 'react';
import { source } from '@/lib/source';

export default function Layout({ children }: { children: ReactNode }) {
  return (
    <DocsLayout
      tree={source.pageTree}
      nav={{ title: 'TrustLoopGuard' }}
      githubUrl="https://github.com/duc/TrustLoopGuard"
    >
      <form action="/api/docs-auth/logout" method="post" className="fixed right-4 top-4 z-50">
        <button
          type="submit"
          className="rounded-md border bg-fd-background px-3 py-1.5 text-sm font-medium text-fd-muted-foreground shadow-sm transition hover:bg-fd-accent hover:text-fd-accent-foreground"
        >
          Log out
        </button>
      </form>
      {children}
    </DocsLayout>
  );
}
