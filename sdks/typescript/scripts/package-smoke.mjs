import { execFileSync, spawnSync } from 'node:child_process';
import { mkdir, mkdtemp, readFile, rename, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const sdkDirectory = dirname(dirname(fileURLToPath(import.meta.url)));
const temporaryDirectory = await mkdtemp(join(tmpdir(), 'featherlane-ai-sdk-'));

try {
  const packed = spawnSync('npm', ['pack', '--json', '--pack-destination', temporaryDirectory], {
    cwd: sdkDirectory,
    encoding: 'utf8',
  });

  if (packed.status !== 0) {
    throw new Error(`npm pack failed:\n${packed.stderr}`);
  }

  const manifestStart = Math.max(0, packed.stdout.lastIndexOf('\n[') + 1);
  const [manifest] = JSON.parse(packed.stdout.slice(manifestStart));
  if (manifest === undefined) {
    throw new Error('npm pack did not return a package manifest');
  }

  const tarball = join(temporaryDirectory, manifest.filename);
  execFileSync('tar', ['-xzf', tarball, '-C', temporaryDirectory]);

  const packageDirectory = join(temporaryDirectory, 'node_modules', '@featherlane-ai', 'sdk');
  await mkdir(dirname(packageDirectory), { recursive: true });
  await rename(join(temporaryDirectory, 'package'), packageDirectory);

  const packageJson = JSON.parse(await readFile(join(packageDirectory, 'package.json'), 'utf8'));
  if (
    packageJson.exports?.['.']?.import !== './dist/index.js' ||
    packageJson.exports?.['.']?.types !== './dist/index.d.ts'
  ) {
    throw new Error('packed SDK export map does not point to dist');
  }

  const imported = spawnSync(
    process.execPath,
    [
      '--input-type=module',
      '--eval',
      "const sdk = await import('@featherlane-ai/sdk');" +
        "if (typeof sdk.guardAgent !== 'function') process.exit(1);" +
        "if (typeof sdk.liveKitRun !== 'function') process.exit(1);" +
        "if (sdk.ToolRegistrationMode?.Strict !== 'strict') process.exit(1);" +
        "if (typeof sdk.GuardedToolBlocked !== 'function') process.exit(1);",
    ],
    {
      cwd: temporaryDirectory,
      encoding: 'utf8',
    },
  );
  if (imported.status !== 0) {
    throw new Error(`packed SDK could not be imported by package name:\n${imported.stderr}`);
  }

  await writeFile(
    join(temporaryDirectory, 'consumer.ts'),
    [
      "import { guardAgent, liveKitRun, ToolRegistrationMode, type AuthorizationDecision, type GuardToolDiscoveryWarning, type LiveKitCloseListener } from '@featherlane-ai/sdk';",
      'const listeners = new Set<LiveKitCloseListener>();',
      'const session = {',
      "  on(_event: 'close', listener: LiveKitCloseListener) { listeners.add(listener); return this; },",
      "  off(_event: 'close', listener: LiveKitCloseListener) { listeners.delete(listener); return this; },",
      '};',
      'const agent = guardAgent({',
      '  tools: { echo: { id: "echo", description: "Echo input", inputSchema: { type: "object" }, async execute(input: { text: string }) { return input.text; } } },',
      '  async reply(message: string) { return message; },',
      '}, {',
      "  agentId: 'smoke-agent',",
      "  baseUrl: 'https://api.example.test',",
      "  run: liveKitRun(session, { externalId: 'RM_smoke' }),",
      '  tools: {',
      '    register: ToolRegistrationMode.Off,',
      '    onDiscoveryWarning(warning: GuardToolDiscoveryWarning) { void warning; },',
      '  },',
      '});',
      'const decision: AuthorizationDecision | undefined = undefined;',
      'void agent;',
      'void decision;',
      '',
    ].join('\n'),
  );
  const typechecked = spawnSync(
    process.execPath,
    [
      join(sdkDirectory, 'node_modules', 'typescript', 'bin', 'tsc'),
      '--noEmit',
      '--strict',
      '--target',
      'ES2022',
      '--module',
      'NodeNext',
      '--moduleResolution',
      'NodeNext',
      'consumer.ts',
    ],
    {
      cwd: temporaryDirectory,
      encoding: 'utf8',
    },
  );
  if (typechecked.status !== 0) {
    throw new Error(
      `packed SDK declarations failed in a TypeScript consumer:\n${typechecked.stdout}${typechecked.stderr}`,
    );
  }

  const sourceFiles = manifest.files.filter(({ path }) => path.startsWith('src/'));
  if (sourceFiles.length > 0) {
    throw new Error(`packed SDK unexpectedly contains ${sourceFiles.length} source files`);
  }

  const generatedRuntimeFiles = manifest.files.filter(
    ({ path }) => path.startsWith('dist/generated/') && path.endsWith('.js'),
  );
  if (generatedRuntimeFiles.length > 0) {
    throw new Error(
      `packed SDK unexpectedly contains ${generatedRuntimeFiles.length} generated runtime files`,
    );
  }

  console.log(
    `Packed SDK smoke test passed: ${manifest.entryCount} files, ${manifest.unpackedSize} bytes`,
  );
} finally {
  await rm(temporaryDirectory, { recursive: true, force: true });
}
