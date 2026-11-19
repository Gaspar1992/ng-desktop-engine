#!/usr/bin/env node
import {
  chmodSync,
  copyFileSync,
  existsSync,
  mkdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { ENGINE_BINARY_PACKAGES } from '../packages/builders-native/src/resolve-engine-binary.mjs';

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT_DIR = resolve(__dirname, '..');
const DIST_ENGINE_DIR = resolve(ROOT_DIR, 'dist/engine-packages');

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

function resolveVersion(args) {
  const explicit = getFlag(args, '--version') || process.env['NG_DESKTOP_APP_VERSION']?.trim();
  if (explicit) return explicit;
  const rootPkg = JSON.parse(readFileSync(resolve(ROOT_DIR, 'package.json'), 'utf8'));
  return rootPkg.version;
}

function stageEnginePackage(targetKey, meta, version, binarySourcePath) {
  const outDir = resolve(DIST_ENGINE_DIR, meta.dirName);
  const binDir = resolve(outDir, 'bin');
  rmSync(outDir, { recursive: true, force: true });
  mkdirSync(binDir, { recursive: true });

  const pkgJson = {
    name: meta.pkgName,
    version,
    description: `Prebuilt native ng-desktop-engine host binary for ${targetKey} (${meta.os}/${meta.cpu})`,
    license: 'MIT',
    os: [meta.os],
    cpu: [meta.cpu],
    ...(meta.libc ? { libc: [meta.libc] } : {}),
    files: ['bin', 'LICENSE', 'README.md'],
    homepage: 'https://github.com/Gaspar1992/ng-desktop-engine',
    bugs: { url: 'https://github.com/Gaspar1992/ng-desktop-engine/issues' },
    repository: {
      type: 'git',
      url: 'https://github.com/Gaspar1992/ng-desktop-engine.git',
    },
    publishConfig: {
      access: 'public',
      provenance: true,
    },
  };
  writeFileSync(resolve(outDir, 'package.json'), `${JSON.stringify(pkgJson, null, 2)}\n`, 'utf8');

  const readme = [
    `# ${meta.pkgName}`,
    '',
    `Prebuilt Rust native rendering runtime binary (\`${meta.binaryName}\`) for **\`${targetKey}\`**.`,
    '',
    'Installed automatically via `optionalDependencies` when you install `@ng-native/builders`.',
    '',
  ].join('\n');
  writeFileSync(resolve(outDir, 'README.md'), readme, 'utf8');

  const licensePath = resolve(ROOT_DIR, 'LICENSE');
  if (existsSync(licensePath)) {
    copyFileSync(licensePath, resolve(outDir, 'LICENSE'));
  }

  if (binarySourcePath) {
    const absBin = resolve(ROOT_DIR, binarySourcePath);
    if (!existsSync(absBin)) {
      throw new Error(`Prebuilt binary not found at '${absBin}' for target '${targetKey}'.`);
    }
    const destBin = resolve(binDir, meta.binaryName);
    copyFileSync(absBin, destBin);
    if (meta.os !== 'win32') {
      chmodSync(destBin, 0o755);
    }
  }

  process.stdout.write(
    `[Engine Package] Staged ${meta.pkgName}@${version} (${targetKey}) -> ${outDir}\n`,
  );
  return outDir;
}

function main() {
  const args = process.argv.slice(2);
  const version = resolveVersion(args);
  const allManifests = args.includes('--all-manifests');
  const target = getFlag(args, '--target');
  const binary = getFlag(args, '--binary');

  if (allManifests) {
    for (const [targetKey, meta] of Object.entries(ENGINE_BINARY_PACKAGES)) {
      stageEnginePackage(targetKey, meta, version, undefined);
    }
    return;
  }

  if (!target || !ENGINE_BINARY_PACKAGES[target]) {
    const valid = Object.keys(ENGINE_BINARY_PACKAGES).join(', ');
    throw new Error(`Missing or unsupported --target='${target}'. Expected one of: ${valid}`);
  }

  stageEnginePackage(target, ENGINE_BINARY_PACKAGES[target], version, binary);
}

try {
  main();
} catch (err) {
  process.stderr.write(
    `[Engine Package Error] ${err instanceof Error ? err.message : String(err)}\n`,
  );
  process.exitCode = 1;
}
