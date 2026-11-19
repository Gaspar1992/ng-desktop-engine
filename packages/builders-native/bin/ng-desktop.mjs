#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const SRC_ROOT = existsSync(resolve(__dirname, '../fesm2022/build-bundle.mjs'))
  ? resolve(__dirname, '../fesm2022')
  : resolve(__dirname, '../src');

const [{ compileBundle, watchBundle }, { analyzeProjectPermissions }, { resolveWorkspaceProject }] =
  await Promise.all([
    import(resolve(SRC_ROOT, 'build-bundle.mjs')),
    import(resolve(SRC_ROOT, 'permission-analyzer.mjs')),
    import(resolve(SRC_ROOT, 'workspace-resolver.mjs')),
  ]);

function getFlag(args, name) {
  const prefix = `${name}=`;
  for (let i = 0; i < args.length; i += 1) {
    if (args[i].startsWith(prefix)) {
      return args[i].slice(prefix.length).trim();
    }
    if (args[i] === name && i + 1 < args.length && !args[i + 1].startsWith('--')) {
      return args[i + 1].trim();
    }
  }
  return undefined;
}

function printHelp() {
  process.stdout.write(
    [
      'ng-desktop — Angular Desktop Engine CLI (Angular CLI + Nx)',
      '',
      'Usage:',
      '  ng-desktop build [--project=<name>] [--config=<path>] [--version=<ver>] [--watch]',
      '  ng-desktop permissions [--project=<name>] [--config=<path>] [--strict]',
      '  ng-desktop package [--project=<name>] [--config=<path>] [--version=<ver>] [--all]',
      '  ng-desktop init [--project=<name>] [--target=<linux-gtk4|windows-winui|macos-appkit>]',
      '',
    ].join('\n'),
  );
}

function handleInit(args) {
  const cwd = process.cwd();
  const targetPlatform = getFlag(args, '--target') || 'linux-gtk4';
  const projectHint = getFlag(args, '--project');
  const nativeConfigBlock = {
    builder: '@ng-native/builders:application',
    options: {
      targetPlatform,
      permissions: { mode: 'auto' },
    },
  };

  const angularJsonPath = resolve(cwd, 'angular.json');
  if (existsSync(angularJsonPath)) {
    const raw = JSON.parse(readFileSync(angularJsonPath, 'utf8'));
    const projectName = projectHint || raw.defaultProject || Object.keys(raw.projects ?? {})[0];
    if (!projectName || !raw.projects?.[projectName]) {
      throw new Error('Could not locate a target project inside angular.json');
    }
    raw.projects[projectName].architect ??= {};
    raw.projects[projectName].architect.native ??= nativeConfigBlock;
    writeFileSync(angularJsonPath, `${JSON.stringify(raw, null, 2)}\n`, 'utf8');
    process.stdout.write(`[ng-desktop init] Configured 'architect.native' for '${projectName}'.\n`);
    return;
  }

  const nxProjectPath = resolve(cwd, 'project.json');
  if (existsSync(nxProjectPath)) {
    const raw = JSON.parse(readFileSync(nxProjectPath, 'utf8'));
    raw.targets ??= {};
    raw.targets.native ??= {
      executor: '@ng-native/builders:application',
      options: nativeConfigBlock.options,
    };
    writeFileSync(nxProjectPath, `${JSON.stringify(raw, null, 2)}\n`, 'utf8');
    process.stdout.write(`[ng-desktop init] Configured Nx 'targets.native' for '${raw.name}'.\n`);
    return;
  }

  throw new Error('No angular.json or project.json found in current working directory.');
}

function main() {
  const args = process.argv.slice(2);
  const cmd = args[0];
  if (!cmd || cmd === '--help' || cmd === '-h') {
    printHelp();
    return;
  }

  const projectName = getFlag(args, '--project');
  const configPath = getFlag(args, '--config');
  const version = getFlag(args, '--version');

  if (cmd === 'build') {
    if (args.includes('--watch')) {
      watchBundle({ configPath, projectName, version });
    } else {
      const result = compileBundle({ configPath, projectName, version, rebuildIcons: true });
      process.stdout.write(
        `[ng-desktop build] Emitted bundle '${result.bundleFile}' and permissions '${result.permissionsFile}'.\n`,
      );
    }
    return;
  }

  if (cmd === 'permissions') {
    const ws = resolveWorkspaceProject({ configPath, projectName });
    const srcRoot = resolve(ws.workspaceRoot, ws.sourceRoot);
    const basePolicy = ws.nativeOptions?.permissions ?? {};
    const policy = args.includes('--strict') ? { ...basePolicy, mode: 'strict' } : basePolicy;
    const manifest = analyzeProjectPermissions(srcRoot, policy);
    process.stdout.write(`${JSON.stringify(manifest, null, 2)}\n`);
    return;
  }

  if (cmd === 'package') {
    const ws = resolveWorkspaceProject({ configPath, projectName });
    const pkgFlag = args.includes('--all') ? '--package-all' : '--package';
    const cargoArgs = [
      'run',
      '--release',
      '--',
      pkgFlag,
      `--config=${ws.configFile}`,
      `--project=${ws.projectName}`,
      `--version=${version || ws.version}`,
    ];
    const run = spawnSync('cargo', cargoArgs, { stdio: 'inherit', cwd: ws.workspaceRoot });
    process.exitCode = run.status ?? 1;
    return;
  }

  if (cmd === 'init') {
    handleInit(args);
    return;
  }

  process.stderr.write(`Unknown command '${cmd}'.\n`);
  printHelp();
  process.exitCode = 1;
}

try {
  main();
} catch (err) {
  process.stderr.write(`[ng-desktop error] ${err instanceof Error ? err.message : String(err)}\n`);
  process.exitCode = 1;
}
