import { type ChatBreakCase } from '../agent-breaker/index';
import {
  chatBreakCases,
  mockProviderReplyFor,
  proxyDemoConfig,
  proxySupportAgent,
} from './config';
import {
  createProxyDemoRuntime,
  sendGatewayChatMessage,
  type AgentChatResult,
  type ProxyDemoRuntime,
} from './runtime';

interface BreakerTurn extends AgentChatResult {
  breakCase: ChatBreakCase;
}

async function main(): Promise<void> {
  const runtime = await createProxyDemoRuntime();
  try {
    printDemoStart(runtime);

    const turns = await runChatAgent(runtime);
    assertProviderSawEveryPrompt(runtime);
    printSummary(turns);
  } finally {
    await runtime.close();
  }
}

async function runChatAgent(runtime: ProxyDemoRuntime): Promise<BreakerTurn[]> {
  const turns: BreakerTurn[] = [];

  for (const breakCase of chatBreakCases) {
    const result = await sendGatewayChatMessage(runtime.route, breakCase.userMessage);
    const turn = { ...result, breakCase };

    assertTurnMatchesBreakCase(turn);
    printTurn(turn);
    turns.push(turn);
  }

  return turns;
}

function assertTurnMatchesBreakCase(turn: BreakerTurn): void {
  const expected = expectedGatewayResult(turn.breakCase);
  const actual = {
    content: turn.content,
    finishReason: turn.finishReason,
    verdict: turn.verdict,
    phase: turn.phase,
  };

  if (JSON.stringify(actual) !== JSON.stringify(expected)) {
    throw new Error(
      `unexpected ${turn.breakCase.label} response: ${JSON.stringify({ expected, actual })}`,
    );
  }

  if (turn.breakCase.expect === 'blocked_output' && !turn.traceId) {
    throw new Error(`blocked ${turn.breakCase.label} did not include a trace id`);
  }
}

function expectedGatewayResult(
  breakCase: ChatBreakCase,
): Pick<AgentChatResult, 'content' | 'finishReason' | 'verdict' | 'phase'> {
  if (breakCase.expect === 'pass_through') {
    return {
      content: mockProviderReplyFor(breakCase),
      finishReason: 'stop',
      verdict: null,
      phase: null,
    };
  }

  return {
    content: proxyDemoConfig.fallbackMessage,
    finishReason: 'content_filter',
    verdict: 'blocked',
    phase: 'output',
  };
}

function assertProviderSawEveryPrompt(runtime: ProxyDemoRuntime): void {
  const expectedPrompts = chatBreakCases.map((breakCase) => breakCase.userMessage);
  const actualPrompts = runtime.provider.calls().map((call) => call.userMessage);

  if (JSON.stringify(actualPrompts) !== JSON.stringify(expectedPrompts)) {
    throw new Error(
      `provider saw unexpected prompts: ${JSON.stringify({ expectedPrompts, actualPrompts })}`,
    );
  }
}

function printDemoStart(runtime: ProxyDemoRuntime): void {
  process.stdout.write('proxy demo: OpenAI-compatible chat agent through TrustLoopGuard\n');
  process.stdout.write(`agent    : ${proxySupportAgent.displayName}\n`);
  process.stdout.write(`workspace : ${runtime.route.workspaceId}\n`);
  process.stdout.write(`route     : ${runtime.gatewayIds.route}\n`);
  process.stdout.write(`baseURL   : ${runtime.route.openAiBaseUrl}\n`);
  process.stdout.write(`provider  : ${runtime.route.providerUrl}\n\n`);
}

function printTurn(turn: BreakerTurn): void {
  process.stdout.write(`chat breaker: ${turn.breakCase.label}\n`);
  process.stdout.write(`  user   : ${turn.breakCase.userMessage}\n`);
  process.stdout.write(`  agent  : ${turn.content}\n`);
  process.stdout.write(`  finish : ${turn.finishReason}\n`);
  process.stdout.write(
    `  guard  : verdict=${turn.verdict ?? 'none'} phase=${turn.phase ?? 'none'} trace=${
      turn.traceId ?? '(none)'
    } latency=${turn.latencyMs} ms\n\n`,
  );
}

function printSummary(turns: readonly BreakerTurn[]): void {
  const latencies = turns.map((turn) => turn.latencyMs).sort((a, b) => a - b);
  const avg = Math.round(latencies.reduce((sum, value) => sum + value, 0) / latencies.length);
  const p95 = percentile(latencies, 0.95);

  process.stdout.write('='.repeat(72) + '\n');
  process.stdout.write(`Gateway proxy: ${turns.length} chat turns, avg=${avg} ms, p95=${p95} ms\n`);
  process.stdout.write('Clean responses have no enforcement headers; blocked output keeps OpenAI shape.\n');
}

function percentile(sorted: readonly number[], fraction: number): number {
  if (sorted.length === 0) return 0;
  const index = Math.min(sorted.length - 1, Math.ceil(sorted.length * fraction) - 1);
  return sorted[index] ?? 0;
}

main().catch((error) => {
  process.stderr.write(`proxy demo failed: ${error instanceof Error ? error.stack : String(error)}\n`);
  process.exitCode = 1;
});
