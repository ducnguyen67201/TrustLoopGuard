import { parseArgs } from 'node:util';

import { HOST_IDS, type CliCommandOptions, type HostId, type TargetSelection } from './types.js';

const COMMANDS = new Set(['doctor', 'install', 'status', 'uninstall']);

export const HELP_TEXT = `TrustLoopGuard coding-agent tool gate

Usage:
  trustloopguard install --agent-id <id> [--url <url>] [--target auto|all|claude,codex,opencode]
  trustloopguard status [--project <path>] [--json]
  trustloopguard doctor [--project <path>] [--json]
  trustloopguard uninstall [--project <path>] [--target <targets> | --all]

The installer reads TLG_API_KEY from the environment and never writes it to disk.`;

function parseTarget(value: string | undefined, fallback: TargetSelection): TargetSelection {
  if (value === undefined || value.trim() === '') return fallback;
  if (value === 'auto' || value === 'all') return value;
  const targets = value.split(',').map((target) => target.trim());
  if (
    targets.length === 0 ||
    new Set(targets).size !== targets.length ||
    targets.some((target) => !HOST_IDS.includes(target as HostId))
  ) {
    throw new Error('--target must be auto, all, or a unique comma-separated host list');
  }
  return targets as HostId[];
}

export function parseCliArgs(argv: string[], cwd: string): CliCommandOptions {
  const parsed = parseArgs({
    args: argv,
    allowPositionals: true,
    strict: true,
    tokens: true,
    options: {
      all: { type: 'boolean' },
      'agent-id': { type: 'string' },
      'allow-unsupported': { type: 'boolean' },
      help: { type: 'boolean', short: 'h' },
      json: { type: 'boolean' },
      project: { type: 'string' },
      target: { type: 'string' },
      url: { type: 'string' },
    },
  });

  const seen = new Set<string>();
  for (const token of parsed.tokens) {
    if (token.kind !== 'option') continue;
    const name = token.name;
    if (seen.has(name)) throw new Error(`--${name} may be specified only once`);
    seen.add(name);
  }

  if (parsed.values.help === true || parsed.positionals.length === 0) {
    return { command: 'help' };
  }
  if (parsed.positionals.length !== 1 || !COMMANDS.has(parsed.positionals[0] ?? '')) {
    throw new Error('expected one command: install, status, doctor, or uninstall');
  }

  const command = parsed.positionals[0] as 'doctor' | 'install' | 'status' | 'uninstall';
  const allowed =
    command === 'install'
      ? new Set(['agent-id', 'allow-unsupported', 'json', 'project', 'target', 'url'])
      : command === 'uninstall'
        ? new Set(['all', 'json', 'project', 'target'])
        : new Set(['json', 'project']);
  for (const option of seen) {
    if (!allowed.has(option)) throw new Error(`--${option} is not valid with ${command}`);
  }

  const project = parsed.values.project ?? cwd;
  const json = parsed.values.json === true;
  if (command === 'install') {
    return {
      command,
      project,
      json,
      agentId: parsed.values['agent-id'],
      allowUnsupported: parsed.values['allow-unsupported'] === true,
      target: parseTarget(parsed.values.target, 'auto'),
      url: parsed.values.url,
    };
  }
  if (command === 'uninstall') {
    const all = parsed.values.all === true;
    if (all && parsed.values.target !== undefined) {
      throw new Error('--all and --target cannot be used together');
    }
    return {
      command,
      project,
      json,
      all,
      target: parseTarget(parsed.values.target, 'all'),
    };
  }
  return { command, project, json };
}
