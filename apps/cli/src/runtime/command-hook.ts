#!/usr/bin/env node
import { dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

import type { HostId, JsonValue, RuntimeEnvironment } from './runtime-types.js';
import {
  authorizeToolCall,
  cancelSessionLeases,
  completeToolCall,
  isManagedProject,
  type BridgeOptions,
} from './bridge.js';
import { parseCommandHookPayload } from './wire.js';

function permissionOutput(decision: 'allow' | 'deny', reason: string): string {
  return JSON.stringify({
    hookSpecificOutput: {
      hookEventName: 'PreToolUse',
      permissionDecision: decision,
      permissionDecisionReason: reason,
    },
  });
}

function parseHost(argv: string[]): HostId {
  const index = argv.indexOf('--host');
  const value = index >= 0 ? argv[index + 1] : undefined;
  if (value === 'claude' || value === 'codex') return value;
  throw new Error('command hook requires --host claude or --host codex');
}

export async function runCommandHook(
  input: string,
  argv: string[],
  cwd: string,
  configRoot: string,
  env: RuntimeEnvironment,
): Promise<{ stdout: string; stderr: string }> {
  const host = parseHost(argv);
  const options: BridgeOptions = { configRoot, env };
  let value: JsonValue;
  try {
    value = JSON.parse(input) as JsonValue;
  } catch {
    if (await shouldFailClosed(host, cwd, configRoot)) {
      return {
        stdout: permissionOutput('deny', 'Featherlane AI could not parse the host hook request.'),
        stderr: '',
      };
    }
    return { stdout: '', stderr: '' };
  }

  let call;
  try {
    call = parseCommandHookPayload(value, host, cwd);
  } catch (error) {
    if (await shouldFailClosed(host, cwd, configRoot)) {
      const reason = error instanceof Error ? error.message : 'invalid hook request';
      return {
        stdout: permissionOutput(
          'deny',
          `Featherlane AI rejected the host hook request: ${reason}.`,
        ),
        stderr: '',
      };
    }
    return { stdout: '', stderr: '' };
  }

  if (call.event === 'pre') {
    const result = await authorizeToolCall(call, options);
    if (!result.managed) return { stdout: '', stderr: '' };
    return {
      stdout: permissionOutput(result.allowed ? 'allow' : 'deny', result.reason),
      stderr: '',
    };
  }
  const completion =
    call.event === 'session-end'
      ? await cancelSessionLeases(call, options)
      : await completeToolCall(
          call,
          call.event === 'post-success' ? 'consumed' : 'canceled',
          options,
        );
  return {
    stdout: '',
    stderr:
      completion.retained === 0
        ? ''
        : `Featherlane AI lease completion failed; ${completion.retained} state file(s) retained: ${completion.errors.join('; ')}\n`,
  };
}

async function shouldFailClosed(host: HostId, cwd: string, configRoot: string): Promise<boolean> {
  try {
    return await isManagedProject(host, cwd, configRoot);
  } catch {
    return true;
  }
}

async function main(): Promise<void> {
  const chunks: Buffer[] = [];
  for await (const chunk of process.stdin) chunks.push(Buffer.from(chunk));
  const configRoot = dirname(dirname(fileURLToPath(import.meta.url)));
  const result = await runCommandHook(
    Buffer.concat(chunks).toString('utf8'),
    process.argv.slice(2),
    process.cwd(),
    configRoot,
    process.env,
  );
  if (result.stdout !== '') process.stdout.write(result.stdout);
  if (result.stderr !== '') process.stderr.write(result.stderr);
}

if (process.argv[1] !== undefined && fileURLToPath(import.meta.url) === process.argv[1]) {
  main().catch((error: Error) => {
    process.stderr.write(`Featherlane AI hook failed: ${error.message}\n`);
    process.exitCode = 1;
  });
}
