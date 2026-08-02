#!/usr/bin/env node
import { parseCliArgs, HELP_TEXT } from './args.js';
import { runCommand } from './commands.js';
import { CliError } from './types.js';

async function main(): Promise<void> {
  let options;
  try {
    options = parseCliArgs(process.argv.slice(2), process.cwd());
  } catch (error) {
    throw new CliError(error instanceof Error ? error.message : 'invalid arguments', 2, HELP_TEXT);
  }
  if (options.command === 'help') {
    process.stdout.write(`${HELP_TEXT}\n`);
    return;
  }
  const exitCode = await runCommand(options, {
    cwd: process.cwd(),
    env: process.env,
    stdout: (message) => process.stdout.write(`${message}\n`),
    stderr: (message) => process.stderr.write(`${message}\n`),
  });
  process.exitCode = exitCode;
}

main().catch((error: Error) => {
  const cliError = error instanceof CliError ? error : new CliError(error.message);
  process.stderr.write(`Featherlane AI: ${cliError.message}\n`);
  if (cliError.remediation !== undefined) process.stderr.write(`${cliError.remediation}\n`);
  process.exitCode = cliError.exitCode;
});
