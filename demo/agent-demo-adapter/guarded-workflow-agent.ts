import { createArenaAdapter, type ArenaAdapterWorkflowResult } from '../arena/adapter';
import { createClient, DEFAULT_AGENT_ID, SERVER_URL, WORKSPACE_ID } from '../shared/env';

import {
  runGuardedWorkflow,
  workflowAgentProfile,
  workflowRunPayload,
  workflowSummary,
  type WorkflowRun,
} from './workflow-agent';

const host = process.env.AGENT_DEMO_WORKFLOW_GUARDED_HOST ?? '127.0.0.1';
const port = Number.parseInt(process.env.AGENT_DEMO_WORKFLOW_GUARDED_PORT ?? '9112', 10);
const agentId = process.env.TL_AGENT_ID ?? DEFAULT_AGENT_ID;

async function main(): Promise<void> {
  const client = createClient();
  const adapter = await createArenaAdapter({
    host,
    port,
    profile: {
      ...workflowAgentProfile,
      displayName: 'TaxPilot Workflow (guarded)',
    },
    async workflow(request) {
      const run = await runGuardedWorkflow({ request, client, agentId });
      return workflowResult(run);
    },
  });

  process.stdout.write('tax workflow guarded adapter: ready\n');
  process.stdout.write(`listen    : ${adapter.url}\n`);
  process.stdout.write(`profile   : ${adapter.url}/arena/profile\n`);
  process.stdout.write(`workflow  : ${adapter.url}/arena/workflow\n`);
  process.stdout.write(`server    : ${SERVER_URL}\n`);
  process.stdout.write(`workspace : ${WORKSPACE_ID ?? '(default runtime workspace)'}\n`);
  process.stdout.write(`agent     : ${agentId}\n\n`);

  const shutdown = (): void => {
    void adapter.close().finally(() => {
      process.exitCode = 0;
    });
  };

  process.once('SIGINT', shutdown);
  process.once('SIGTERM', shutdown);
}

function workflowResult(run: WorkflowRun): ArenaAdapterWorkflowResult {
  const firstTrace = run.blockedActions.find((action) => action.guardDecision?.traceId)?.guardDecision
    ?.traceId;

  return {
    content: workflowSummary(run),
    finishReason: run.blockedActions.length > 0 ? 'content_filter' : 'stop',
    verdict: run.blockedActions.length > 0 ? 'blocked' : null,
    phase: run.blockedActions.length > 0 ? 'tool' : null,
    traceId: firstTrace ?? null,
    result: workflowRunPayload(run),
  };
}

main().catch((error) => {
  process.stderr.write(
    `tax workflow guarded adapter failed: ${error instanceof Error ? error.stack : String(error)}\n`,
  );
  process.exitCode = 1;
});
