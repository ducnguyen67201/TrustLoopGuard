import { IconCircleCheck, IconCircleDashed } from '@tabler/icons-react';

export function SetupRunway({ hasServer, hasAssignments }: { hasServer: boolean; hasAssignments: boolean }) {
  const steps = [
    { label: 'Connect server', complete: hasServer, detail: 'Register and synchronize a remote Streamable HTTP MCP server.' },
    { label: 'Assign tools', complete: hasAssignments, locked: !hasServer, detail: 'Choose which members can discover and call each pinned tool.' },
    { label: 'Share endpoint', complete: hasAssignments, locked: !hasAssignments, detail: 'Members connect their AI client to the managed workspace endpoint.' },
  ];
  return <ol className="grid gap-3 md:grid-cols-3">{steps.map((step) => <li key={step.label} className="rounded-lg border border-border p-4">{step.complete ? <IconCircleCheck className="text-primary" aria-label="Complete" /> : <IconCircleDashed className="text-muted-foreground" aria-label={step.locked ? 'Locked' : 'Ready'} />}<p className="mt-2 font-medium text-foreground">{step.label}</p><p className="mt-1 text-sm text-muted-foreground">{step.locked ? `Complete the prior step first. ${step.detail}` : step.detail}</p></li>)}</ol>;
}
