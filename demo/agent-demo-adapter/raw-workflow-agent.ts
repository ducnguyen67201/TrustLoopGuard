import { createArenaAdapter, type ArenaAdapterWorkflowResult } from '../arena/adapter';

import {
  runUnguardedWorkflow,
  workflowAgentProfile,
  workflowRunPayload,
  workflowSummary,
  type WorkflowRun,
} from './workflow-agent';

const host = process.env.AGENT_DEMO_WORKFLOW_RAW_HOST ?? '127.0.0.1';
const port = Number.parseInt(process.env.AGENT_DEMO_WORKFLOW_RAW_PORT ?? '9111', 10);

async function main(): Promise<void> {
  const adapter = await createArenaAdapter({
    host,
    port,
    profile: {
      ...workflowAgentProfile,
      displayName: 'TaxPilot Workflow (raw)',
    },
    async workflow(request) {
      const run = await runUnguardedWorkflow(request);
      return workflowResult(run);
    },
  });

  process.stdout.write('tax workflow raw adapter: ready\n');
  process.stdout.write(`listen   : ${adapter.url}\n`);
  process.stdout.write(`profile  : ${adapter.url}/arena/profile\n`);
  process.stdout.write(`workflow : ${adapter.url}/arena/workflow\n\n`);

  const shutdown = (): void => {
    void adapter.close().finally(() => {
      process.exitCode = 0;
    });
  };

  process.once('SIGINT', shutdown);
  process.once('SIGTERM', shutdown);
}

function workflowResult(run: WorkflowRun): ArenaAdapterWorkflowResult {
  return {
    content: workflowSummary(run),
    finishReason: 'stop',
    verdict: null,
    phase: null,
    traceId: null,
    result: workflowRunPayload(run),
  };
}

main().catch((error) => {
  process.stderr.write(
    `tax workflow raw adapter failed: ${error instanceof Error ? error.stack : String(error)}\n`,
  );
  process.exitCode = 1;
});
