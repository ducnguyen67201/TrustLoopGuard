import { DocsLayout } from 'fumadocs-ui/layouts/docs';
import type { ReactNode } from 'react';
import { source } from '@/lib/source';

export default function Layout({ children }: { children: ReactNode }) {
  return (
    <DocsLayout
      tree={source.pageTree}
      nav={{ title: 'TrustLoopGuard' }}
      links={[
        {
          type: 'custom',
          on: 'nav',
          secondary: true,
          children: (
            <form action="/api/docs-auth/logout" method="post">
              <button
                type="submit"
                className="rounded-md border px-3 py-1.5 text-sm font-medium text-fd-muted-foreground transition hover:bg-fd-accent hover:text-fd-accent-foreground"
              >
                Log out
              </button>
            </form>
          ),
        },
      ]}
      githubUrl="https://github.com/duc/TrustLoopGuard"
    >
      {children}
    </DocsLayout>
  );
}
