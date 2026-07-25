import { execFileSync, spawnSync } from 'node:child_process';
import { chmod, mkdir, mkdtemp, readFile, rename, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { delimiter, dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const packageDirectory = dirname(dirname(fileURLToPath(import.meta.url)));
const temporaryDirectory = await mkdtemp(join(tmpdir(), 'trustloopguard-cli-package-'));

try {
  const packed = spawnSync('npm', ['pack', '--json', '--pack-destination', temporaryDirectory], {
    cwd: packageDirectory,
    encoding: 'utf8',
  });
  if (packed.status !== 0) throw new Error(`npm pack failed:\n${packed.stderr}`);
  const manifestStart = Math.max(0, packed.stdout.lastIndexOf('\n[') + 1);
  const [manifest] = JSON.parse(packed.stdout.slice(manifestStart));
  if (manifest === undefined) throw new Error('npm pack did not return a package manifest');

  execFileSync('tar', [
    '-xzf',
    join(temporaryDirectory, manifest.filename),
    '-C',
    temporaryDirectory,
  ]);
  const installedPackage = join(temporaryDirectory, 'node_modules', '@trustloopguard', 'cli');
  await mkdir(dirname(installedPackage), { recursive: true });
  await rename(join(temporaryDirectory, 'package'), installedPackage);

  const packageJson = JSON.parse(await readFile(join(installedPackage, 'package.json'), 'utf8'));
  if (packageJson.bin?.trustloopguard !== 'dist/index.js') {
    throw new Error('packed CLI bin does not point to dist/index.js');
  }
  const paths = manifest.files.map((entry) => entry.path);
  for (const required of [
    'LICENSE',
    'README.md',
    'dist/index.js',
    'dist/runtime/bridge.js',
    'dist/runtime/command-hook.js',
    'dist/runtime/opencode-plugin.js',
  ]) {
    if (!paths.includes(required)) throw new Error(`packed CLI is missing ${required}`);
  }
  if (paths.some((path) => path.startsWith('src/') || path.endsWith('.test.js'))) {
    throw new Error('packed CLI unexpectedly contains source or test files');
  }

  const home = join(temporaryDirectory, 'home');
  const config = join(temporaryDirectory, 'config');
  const project = join(temporaryDirectory, 'project');
  const bin = join(temporaryDirectory, 'bin');
  await Promise.all([mkdir(home), mkdir(config), mkdir(project), mkdir(bin)]);
  const fakeClaude = join(bin, process.platform === 'win32' ? 'claude.cmd' : 'claude');
  await writeFile(
    fakeClaude,
    process.platform === 'win32'
      ? '@echo off\r\necho 2.1.133 (Claude Code)\r\n'
      : '#!/bin/sh\necho 2.1.133 Claude Code\n',
  );
  if (process.platform !== 'win32') await chmod(fakeClaude, 0o700);
  const env = {
    ...process.env,
    HOME: home,
    XDG_CONFIG_HOME: config,
    PATH: `${bin}${delimiter}${process.env.PATH ?? ''}`,
    TLG_API_KEY: 'tl_live_package_smoke_only',
  };
  const cli = join(installedPackage, 'dist', 'index.js');

  const help = spawnSync(process.execPath, [cli, '--help'], { encoding: 'utf8', env });
  if (help.status !== 0 || !help.stdout.includes('trustloopguard install')) {
    throw new Error(`packed CLI help failed:\n${help.stderr}`);
  }
  const install = spawnSync(
    process.execPath,
    [
      cli,
      'install',
      '--agent-id',
      'package-agent',
      '--url',
      'https://api.example.test',
      '--target',
      'claude',
      '--project',
      project,
      '--json',
    ],
    { encoding: 'utf8', env },
  );
  if (install.status !== 0) {
    throw new Error(`packed CLI install failed:\n${install.stdout}\n${install.stderr}`);
  }
  const registryText = await readFile(join(config, 'trustloopguard', 'registry.json'), 'utf8');
  if (registryText.includes(env.TLG_API_KEY)) throw new Error('runtime key leaked into registry');

  const status = spawnSync(process.execPath, [cli, 'status', '--project', project, '--json'], {
    encoding: 'utf8',
    env,
  });
  if (status.status !== 0 || !status.stdout.includes('"registered": true')) {
    throw new Error(`packed CLI status failed:\n${status.stderr}`);
  }
  const uninstall = spawnSync(
    process.execPath,
    [cli, 'uninstall', '--project', project, '--all', '--json'],
    { encoding: 'utf8', env },
  );
  if (uninstall.status !== 0) throw new Error(`packed CLI uninstall failed:\n${uninstall.stderr}`);

  console.log(
    `Packed CLI smoke test passed: ${manifest.entryCount} files, ${manifest.unpackedSize} bytes`,
  );
} finally {
  await rm(temporaryDirectory, { recursive: true, force: true });
}
