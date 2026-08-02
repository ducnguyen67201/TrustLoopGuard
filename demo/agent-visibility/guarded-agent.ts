import {
  GuardedToolBlocked,
  ToolRegistrationMode,
  guardAgent,
  type AgentProfile,
  type RunDetail,
} from '@featherlane-ai/sdk';

import { createClient } from '../shared/env';

const AGENT_ID = process.env.TL_AGENT_ID?.trim() || 'demo-agent-visibility';
const TRACE_WAIT_ATTEMPTS = 20;
const TRACE_WAIT_MS = 250;

type AppointmentInput = {
  customer: string;
  partySize: number;
  requestedTime: string;
};

type Appointment = AppointmentInput & {
  confirmationId: string;
};

const appointments: Appointment[] = [];

const bookAppointment = {
  id: 'book_appointment',
  description: 'Book a customer appointment for a requested time and party size.',
  inputSchema: {
    type: 'object',
    additionalProperties: false,
    properties: {
      customer: { type: 'string' },
      partySize: { type: 'integer', minimum: 1 },
      requestedTime: { type: 'string' },
    },
    required: ['customer', 'partySize', 'requestedTime'],
  },
  async execute(input: AppointmentInput): Promise<Appointment> {
    const appointment = {
      ...input,
      confirmationId: `appt-${appointments.length + 1}`,
    };
    appointments.push(appointment);
    return appointment;
  },
};

const rawAgent = {
  tools: { bookAppointment },
  async reply(message: string): Promise<string> {
    const partySize = readPartySize(message);
    const appointment = await this.tools.bookAppointment.execute({
      customer: 'Browser QA customer',
      partySize,
      requestedTime: '2026-07-17T10:00:00-07:00',
    });
    return [
      `Appointment ${appointment.confirmationId} is booked`,
      `for ${appointment.customer}`,
      `at ${appointment.requestedTime}`,
      `for ${appointment.partySize} people.`,
    ].join(' ');
  },
};

async function main(): Promise<void> {
  const client = createClient();
  await client.upsertAgent(agentProfile());

  const agent = guardAgent(rawAgent, {
    agentId: AGENT_ID,
    client,
    failClosed: true,
    tools: {
      register: ToolRegistrationMode.Strict,
      inferMetadata: () => ({
        side_effect: 'api_mutation',
        reversible: true,
        params: [
          {
            path: 'customer',
            role: 'content_bearing',
            allowed_sources: [{ origin: 'user' }],
          },
          {
            path: 'partySize',
            role: 'content_bearing',
            allowed_sources: [{ origin: 'user' }],
          },
          {
            path: 'requestedTime',
            role: 'content_bearing',
            allowed_sources: [{ origin: 'user' }],
          },
        ],
      }),
      onDiscoveryWarning: (warning) => {
        process.stderr.write(`tool discovery warning: ${warning.code}: ${warning.message}\n`);
      },
    },
  });

  const message =
    process.argv
      .slice(2)
      .filter((argument) => argument !== '--')
      .join(' ')
      .trim() || 'Book an appointment for 2 people.';
  const result = await client.withRun(
    {
      agentId: AGENT_ID,
      kind: 'chat_session',
      externalId: `agent-visibility-${Date.now()}`,
      inputSummary: message,
      metadata: { demo: 'agent-visibility', integration: 'guardAgent' },
    },
    async (run) => {
      const reply = await run.withEvent(
        {
          kind: 'assistant_turn',
          label: 'guarded_agent_reply',
          input_summary: message,
          metadata: { tool: bookAppointment.id },
        },
        () => agent.reply(message),
      );
      return { reply, runId: run.id };
    },
  );

  const detail = await waitForRunVisibility(client, result.runId);
  const eventKinds = new Set(detail.traces.map((trace) => traceEventKind(trace)));
  if (!eventKinds.has('tool.call.proposed') || !eventKinds.has('output.proposed')) {
    throw new Error(
      `expected tool and output traces for run ${result.runId}; received ${[...eventKinds].join(', ')}`,
    );
  }

  process.stdout.write(
    `${JSON.stringify(
      {
        agentId: AGENT_ID,
        runId: result.runId,
        reply: result.reply,
        toolExecutions: appointments.length,
        traces: detail.traces.map((trace) => ({
          id: trace.trace_id,
          kind: traceEventKind(trace),
          domain: trace.domain,
          decision: trace.decision,
          runEventId: trace.run_event_id,
        })),
      },
      null,
      2,
    )}\n`,
  );
}

function traceEventKind(trace: RunDetail['traces'][number]): string {
  const event = trace.payload['event'];
  if (event === null || typeof event !== 'object') return 'unknown';
  const kind = Reflect.get(event, 'kind', event);
  return typeof kind === 'string' ? kind : 'unknown';
}

function agentProfile(): AgentProfile {
  return {
    agent_id: AGENT_ID,
    display_name: 'Guarded appointment agent',
    scope: {
      in_scope: ['appointment scheduling'],
      out_of_scope: ['medical advice', 'payment processing'],
    },
    authority: {
      can_promise: ['confirmed appointment times returned by the booking tool'],
      cannot_promise: ['availability that the booking tool did not confirm'],
    },
    tone: {
      target: 'clear and concise',
      forbidden: ['dismissive', 'deceptive'],
    },
    knowledge_sources: [],
    escalation_triggers: ['payment request', 'medical request'],
    workflow_requirements: [],
    system_prompt:
      'Schedule appointments only through book_appointment and report its confirmed result.',
  };
}

function readPartySize(message: string): number {
  const matched = message.match(/\b(\d+)\b/);
  return matched === null ? 2 : Number.parseInt(matched[1] ?? '2', 10);
}

async function waitForRunVisibility(
  client: ReturnType<typeof createClient>,
  runId: string,
): Promise<RunDetail> {
  for (let attempt = 0; attempt < TRACE_WAIT_ATTEMPTS; attempt += 1) {
    const detail = await client.getRun(runId);
    if (detail.traces.length >= 2) return detail;
    await new Promise((resolve) => setTimeout(resolve, TRACE_WAIT_MS));
  }
  throw new Error(`run ${runId} did not persist both guard traces in time`);
}

main().catch((error) => {
  if (error instanceof GuardedToolBlocked) {
    process.stderr.write(
      `sample tool blocked: ${error.tool.name} -> ${error.decision.effect} (${error.decision.reason})\n`,
    );
  } else {
    process.stderr.write(
      `agent visibility demo failed: ${error instanceof Error ? error.message : String(error)}\n`,
    );
  }
  process.exitCode = 1;
});
