import { IconArrowRight, IconLockAccess, IconPlugConnected } from '@tabler/icons-react';
import type { McpGatewayConnection } from '@featherlane-ai/sdk';

import { Badge } from '@/components/ui/badge';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';

export function SwitchyardMap({ connections }: { connections: McpGatewayConnection[] }) {
  return (
    <Card>
      <CardHeader><CardTitle>Managed tool route</CardTitle></CardHeader>
      <CardContent className="grid gap-3 lg:grid-cols-[1fr_auto_1fr_auto_1fr] lg:items-center">
        <div className="grid gap-2" aria-label="Upstream MCP servers">
          {connections.map((connection) => <div key={connection.id} className="rounded-lg border border-border p-3"><p className="font-medium text-foreground">{connection.display_name}</p><Badge variant="outline">{connection.enabled ? connection.last_sync_status : 'disabled'}</Badge></div>)}
        </div>
        <IconArrowRight className="hidden text-muted-foreground lg:block" aria-hidden />
        <div className="rounded-lg border border-border p-3 text-center"><IconLockAccess className="mx-auto text-primary" aria-hidden /><p className="mt-2 font-medium">Tool access</p><p className="text-xs text-muted-foreground">Per-member assignments</p></div>
        <IconArrowRight className="hidden text-muted-foreground lg:block" aria-hidden />
        <div className="rounded-lg border border-border p-3 text-center"><IconPlugConnected className="mx-auto text-primary" aria-hidden /><p className="mt-2 font-medium">Runtime policy</p><p className="text-xs text-muted-foreground">Checks every permitted call</p></div>
      </CardContent>
    </Card>
  );
}
