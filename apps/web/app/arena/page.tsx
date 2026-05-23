import type { Metadata } from 'next';

import { ArenaPageContent } from './page-content';

export const metadata: Metadata = {
  title: 'Agent Breakaway Arena | TrustLoopGuard',
  description: 'Open demo arena for comparing raw and TrustLoopGuard-protected chat agents.',
};

export default function ArenaPage() {
  return <ArenaPageContent />;
}
