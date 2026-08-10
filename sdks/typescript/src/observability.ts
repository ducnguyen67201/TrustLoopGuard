import {
  SpanStatusCode,
  context,
  createContextKey,
  type Context,
  type Span,
  type Tracer,
} from '@opentelemetry/api';
import { OTLPTraceExporter } from '@opentelemetry/exporter-trace-otlp-proto';
import { resourceFromAttributes } from '@opentelemetry/resources';
import {
  BatchSpanProcessor,
  type ReadableSpan,
  type SpanProcessor,
} from '@opentelemetry/sdk-trace-base';
import { NodeTracerProvider } from '@opentelemetry/sdk-trace-node';

import {
  Client,
  runCorrelation,
  type ActiveRun,
  type ClientOptions,
  type RunCorrelationContext,
  type WithRunOptions,
} from './client.js';
import type { CreateRunEventRequest } from './generated/CreateRunEventRequest.js';
import type { RunEventSummary } from './generated/RunEventSummary.js';

const DEFAULT_API_URL = 'https://api.featherlane.ai';
const RUN_CONTEXT = createContextKey('featherlane.run.correlation');
const EVENT_SPAN_NAMES = {
  user_turn: 'agent.user_turn',
  assistant_turn: 'agent.assistant_turn',
  tool_call: 'agent.tool_call',
  workflow_step: 'agent.workflow_step',
  interruption: 'agent.interruption',
  retry: 'agent.retry',
  system_event: 'agent.system_event',
  other: 'agent.event',
} as const;

type RunEventInput = Omit<CreateRunEventRequest, 'metadata'> & {
  metadata?: Record<string, unknown>;
};

export type ObservedRunOptions = Omit<WithRunOptions, 'agentId'>;

export interface ObservabilityInitOptions extends Omit<
  ClientOptions,
  'apiKey' | 'baseUrl' | 'runTelemetry'
> {
  agentId: string;
  apiKey?: string;
  baseUrl?: string;
  serviceName?: string;
}

export interface ObservedRun {
  id: string;
  client: Client;
  event(request: RunEventInput): Promise<RunEventSummary>;
  withEvent<T>(request: RunEventInput, operation: () => Promise<T>): Promise<T>;
}

export interface ObservedRunResult<T> {
  runId: string;
  value: T;
}

export interface ObservedAgent {
  client: Client;
  run<T>(
    options: ObservedRunOptions,
    operation: (run: ObservedRun) => Promise<T>,
  ): Promise<ObservedRunResult<T>>;
  shutdown(): Promise<void>;
}

/**
 * Node.js OpenTelemetry integration for one or more Runs owned by one agent.
 * Runtime credentials default to FEATHERLANE_AI_API_KEY and
 * FEATHERLANE_AI_URL, so most applications only provide agentId.
 */
export const observability = {
  init(options: ObservabilityInitOptions): ObservedAgent {
    const baseUrl = options.baseUrl ?? process.env['FEATHERLANE_AI_URL'] ?? DEFAULT_API_URL;
    const apiKey = options.apiKey ?? process.env['FEATHERLANE_AI_API_KEY'];
    const exporter = new OTLPTraceExporter({
      url: new URL('/v1/otel/v1/traces', trailingSlash(baseUrl)).toString(),
      headers: apiKey === undefined ? {} : { authorization: `Bearer ${apiKey}` },
    });
    const provider = new NodeTracerProvider({
      resource: resourceFromAttributes({
        'service.name': options.serviceName ?? options.agentId,
      }),
      spanProcessors: [new RunCorrelationSpanProcessor(), new BatchSpanProcessor(exporter)],
    });
    provider.register();
    const tracer = provider.getTracer('featherlane-observability');
    const client = new Client({
      baseUrl,
      ...(apiKey === undefined ? {} : { apiKey }),
      ...(options.fetchImpl === undefined ? {} : { fetchImpl: options.fetchImpl }),
      ...(options.retry === undefined ? {} : { retry: options.retry }),
      ...(options.onRetry === undefined ? {} : { onRetry: options.onRetry }),
      ...(options.telemetryFlushTimeoutMs === undefined
        ? {}
        : { telemetryFlushTimeoutMs: options.telemetryFlushTimeoutMs }),
      ...(options.onRunLifecycleWarning === undefined
        ? {}
        : { onRunLifecycleWarning: options.onRunLifecycleWarning }),
      runTelemetry: {
        bindRun(correlation) {
          const span = tracer.startSpan('agent.run.started', {
            attributes: correlation.attributes,
          });
          span.end();
        },
        async forceFlush(correlation) {
          const span = tracer.startSpan('agent.telemetry.flush', {
            attributes: correlation.attributes,
          });
          span.end();
          await provider.forceFlush();
        },
      },
    });
    let shutdownPromise: Promise<void> | undefined;

    return {
      client,
      run: (runOptions, operation) =>
        observeRun(client, tracer, options.agentId, runOptions, operation),
      shutdown() {
        if (shutdownPromise === undefined) shutdownPromise = provider.shutdown();
        return shutdownPromise;
      },
    };
  },
};

async function observeRun<T>(
  client: Client,
  tracer: Tracer,
  agentId: string,
  options: ObservedRunOptions,
  operation: (run: ObservedRun) => Promise<T>,
): Promise<ObservedRunResult<T>> {
  return client.withRun({ ...options, agentId }, async (activeRun) => {
    const correlation = runCorrelation(activeRun.id, agentId);
    const runContext = context.active().setValue(RUN_CONTEXT, correlation);
    return context.with(runContext, () =>
      tracer.startActiveSpan('agent.run', { attributes: correlation.attributes }, async (span) => {
        try {
          const value = await operation(observedRun(client, tracer, activeRun));
          return { runId: activeRun.id, value };
        } catch (error) {
          recordSpanError(span, error instanceof Error ? error : new Error(String(error)));
          throw error;
        } finally {
          span.end();
        }
      }),
    );
  });
}

function observedRun(client: Client, tracer: Tracer, activeRun: ActiveRun): ObservedRun {
  return {
    id: activeRun.id,
    client,
    event: (request) =>
      observedEvent(tracer, request, () => client.createRunEvent(activeRun.id, request)),
    withEvent: (request, operation) =>
      observedEvent(tracer, request, () => activeRun.withEvent(request, operation)),
  };
}

function observedEvent<T>(
  tracer: Tracer,
  request: RunEventInput,
  operation: () => Promise<T>,
): Promise<T> {
  const name = EVENT_SPAN_NAMES[request.kind];
  return tracer.startActiveSpan(
    name,
    {
      attributes: {
        'featherlane.run.event.kind': request.kind,
        ...(request.label === undefined || request.label === null
          ? {}
          : { 'featherlane.run.event.label': request.label }),
      },
    },
    async (span) => {
      try {
        return await operation();
      } catch (error) {
        recordSpanError(span, error instanceof Error ? error : new Error(String(error)));
        throw error;
      } finally {
        span.end();
      }
    },
  );
}

class RunCorrelationSpanProcessor implements SpanProcessor {
  onStart(span: Span, parentContext: Context): void {
    const correlation = parentContext.getValue(RUN_CONTEXT);
    if (!isRunCorrelation(correlation)) return;
    span.setAttributes(correlation.attributes);
  }

  onEnd(_span: ReadableSpan): void {}
  forceFlush(): Promise<void> {
    return Promise.resolve();
  }
  shutdown(): Promise<void> {
    return Promise.resolve();
  }
}

function isRunCorrelation(value: ReturnType<Context['getValue']>): value is RunCorrelationContext {
  return (
    typeof value === 'object' &&
    value !== null &&
    Reflect.has(value, 'runId') &&
    Reflect.has(value, 'agentId') &&
    Reflect.has(value, 'attributes')
  );
}

function recordSpanError(span: Span, error: Error): void {
  span.recordException(error);
  span.setStatus({ code: SpanStatusCode.ERROR, message: error.message });
}

function trailingSlash(value: string): string {
  return value.endsWith('/') ? value : `${value}/`;
}
