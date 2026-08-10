import { describe, expect, it, vi } from 'vitest';

import { observability } from '../src/observability';
import { jsonResponse, mockFetch } from './test-utils';

const RUN_ID = '018f1111-1111-7111-8111-111111111111';
const EVENT_ID = '018f2222-2222-7222-8222-222222222222';

const telemetry = vi.hoisted(() => {
  const spanNames: string[] = [];
  const exporterOptions: Array<{ url: string; headers: Record<string, string> }> = [];
  const forceFlush = vi.fn(async () => undefined);
  const shutdown = vi.fn(async () => undefined);
  const tracer = {
    startSpan(name: string) {
      spanNames.push(name);
      return {
        end: vi.fn(),
        recordException: vi.fn(),
        setStatus: vi.fn(),
      };
    },
    async startActiveSpan<T>(
      name: string,
      _options: object,
      callback: (span: {
        end(): void;
        recordException(error: Error): void;
        setStatus(status: object): void;
      }) => Promise<T>,
    ): Promise<T> {
      spanNames.push(name);
      return callback({
        end: vi.fn(),
        recordException: vi.fn(),
        setStatus: vi.fn(),
      });
    },
  };
  return { exporterOptions, forceFlush, shutdown, spanNames, tracer };
});

vi.mock('@opentelemetry/api', () => ({
  context: {
    active: () => ({ setValue: vi.fn().mockReturnThis() }),
    with: async <T>(_context: object, callback: () => Promise<T>) => callback(),
  },
  createContextKey: (name: string) => Symbol.for(name),
  SpanStatusCode: { ERROR: 2 },
}));

vi.mock('@opentelemetry/exporter-trace-otlp-proto', () => ({
  OTLPTraceExporter: class {
    constructor(options: { url: string; headers: Record<string, string> }) {
      telemetry.exporterOptions.push(options);
    }
  },
}));

vi.mock('@opentelemetry/resources', () => ({
  resourceFromAttributes: (attributes: Record<string, string>) => attributes,
}));

vi.mock('@opentelemetry/sdk-trace-base', () => ({
  BatchSpanProcessor: class {},
}));

vi.mock('@opentelemetry/sdk-trace-node', () => ({
  NodeTracerProvider: class {
    forceFlush = telemetry.forceFlush;
    shutdown = telemetry.shutdown;
    getTracer() {
      return telemetry.tracer;
    }
    register() {}
  },
}));

function runBody(status = 'running') {
  return {
    id: RUN_ID,
    workspace_id: 'ws_test',
    environment_id: 'production',
    environment: 'production',
    agent_id: 'support-agent',
    kind: 'chat_session',
    status,
    external_id: 'chat-123',
    metadata: {},
    started_at: '2026-05-17T00:00:00Z',
    ended_at: status === 'running' ? null : '2026-05-17T00:01:00Z',
    created_at: '2026-05-17T00:00:00Z',
    updated_at: '2026-05-17T00:00:00Z',
    trace_count: 0,
    blocked_count: 0,
    rewritten_count: 0,
    escalated_count: 0,
    p95_latency_ms: null,
  };
}

describe('observability', () => {
  it('initializes once and captures a complete Run through one readable boundary', async () => {
    const fetchSpy = mockFetch(async (input) => {
      const url = String(input);
      if (url.endsWith('/v1/runs')) return jsonResponse(runBody(), 201);
      if (url.endsWith('/events')) {
        return jsonResponse({
          id: EVENT_ID,
          workspace_id: 'ws_test',
          run_id: RUN_ID,
          agent_id: 'support-agent',
          sequence: 1,
          kind: 'user_turn',
          label: null,
          input_summary: 'Where is order 42?',
          output_summary: null,
          metadata: {},
          occurred_at: '2026-05-17T00:00:01Z',
          created_at: '2026-05-17T00:00:01Z',
        });
      }
      if (url.endsWith('/finalize')) {
        return jsonResponse({
          run: runBody('completed'),
          finalization: {
            finalized_at: '2026-05-17T00:01:00Z',
            boundary_source: 'explicit_sdk',
            boundary_confidence: 'authoritative',
            capture_status: 'waiting',
            capture_deadline: '2026-05-17T00:01:30Z',
            expected_flush_id: 'flush-id',
          },
          evaluation_status: 'waiting_capture',
        });
      }
      throw new Error(`Unexpected request: ${url}`);
    });

    const observed = observability.init({
      agentId: 'support-agent',
      baseUrl: 'http://server.test',
      apiKey: 'runtime-key',
      fetchImpl: fetchSpy,
    });

    const result = await observed.run({ externalId: 'chat-123' }, async (run) => {
      await run.event({ kind: 'user_turn', input_summary: 'Where is order 42?' });
      return 'Order 42 is in transit.';
    });
    await observed.shutdown();

    expect(result).toEqual({ runId: RUN_ID, value: 'Order 42 is in transit.' });
    expect(fetchSpy.mock.calls.map(([url]) => String(url))).toEqual([
      'http://server.test/v1/runs',
      `http://server.test/v1/runs/${RUN_ID}/events`,
      `http://server.test/v1/runs/${RUN_ID}/finalize`,
    ]);
    expect(telemetry.exporterOptions).toEqual([
      {
        url: 'http://server.test/v1/otel/v1/traces',
        headers: { authorization: 'Bearer runtime-key' },
      },
    ]);
    expect(telemetry.spanNames).toEqual([
      'agent.run.started',
      'agent.run',
      'agent.user_turn',
      'agent.telemetry.flush',
    ]);
    expect(telemetry.forceFlush).toHaveBeenCalledOnce();
    expect(telemetry.shutdown).toHaveBeenCalledOnce();
  });
});
