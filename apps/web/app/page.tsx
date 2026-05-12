import { AppShell } from '@/components/AppShell';
import { Playground } from '../components/playground/Playground';
import { getServerUrl } from '../lib/server-url';

export default function Home() {
  const serverUrl = getServerUrl();

  return (
    <AppShell
      title="Playground"
      description="Submit a guardrail check and inspect the decision payload returned by tl-server."
      active="playground"
      footer={
        <>
          <span>POST {serverUrl}/v1/check</span>
          <span>override via NEXT_PUBLIC_TL_SERVER_URL</span>
        </>
      }
    >
      <Playground />
    </AppShell>
  );
}
