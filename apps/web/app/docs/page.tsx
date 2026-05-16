import { redirect } from 'next/navigation';

import { env } from '@/env';

export default function DocsPage() {
  redirect(env.NEXT_PUBLIC_DOCS_URL);
}
