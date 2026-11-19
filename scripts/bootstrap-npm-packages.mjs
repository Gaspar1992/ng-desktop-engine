#!/usr/bin/env node
import { execFileSync } from 'node:child_process';
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT_DIR = resolve(fileURLToPath(new URL('..', import.meta.url)));
const DIST_PACKAGES_DIR = resolve(ROOT_DIR, 'dist/packages');
const DIST_ENGINE_DIR = resolve(ROOT_DIR, 'dist/engine-packages');

const PACKAGE_DIRS_ORDER = [
  'platform-native',
  'forms-native',
  'router-native',
  'http-native',
  'desktop-native',
  'builders-native',
];

const PUBLISH_PRIORITY = [
  'engine-',
  'platform-',
  'forms-',
  'router-',
  'http-',
  'desktop-',
  'builders-',
];

function getFlagValue(args, flag) {
  const prefix = `${flag}=`;
  for (let i = 0; i < args.length; i += 1) {
    if (args[i].startsWith(prefix)) {
      return args[i].slice(prefix.length).trim();
    }
    if (args[i] === flag && i + 1 < args.length && !args[i + 1].startsWith('--')) {
      return args[i + 1].trim();
    }
  }
  return undefined;
}

function runNpm(npmArgs, { capture = false } = {}) {
  return execFileSync('npm', npmArgs, {
    cwd: ROOT_DIR,
    shell: process.platform === 'win32',
    encoding: 'utf8',
    stdio: capture ? 'pipe' : 'inherit',
  });
}

function readTarballPackageJson(tarballPath) {
  const raw = execFileSync('tar', ['-xOf', tarballPath, 'package/package.json'], {
    encoding: 'utf8',
  });
  return JSON.parse(raw);
}

function tarballSortRank(filename) {
  const base = filename.replace(/^ng-desktop-/, '');
  const idx = PUBLISH_PRIORITY.findIndex((prefix) => base.startsWith(prefix));
  return idx === -1 ? PUBLISH_PRIORITY.length : idx;
}

function collectFromTarballDir(tarballDir) {
  const absDir = resolve(ROOT_DIR, tarballDir);
  if (!existsSync(absDir)) {
    throw new Error(`Tarball directory not found: '${absDir}'`);
  }
  const tarballs = readdirSync(absDir)
    .filter((f) => f.startsWith('ng-desktop-') && f.endsWith('.tgz'))
    .sort((a, b) => tarballSortRank(a) - tarballSortRank(b) || a.localeCompare(b));

  if (tarballs.length === 0) {
    throw new Error(`No 'ng-desktop-*.tgz' files found in '${absDir}'`);
  }

  return tarballs.map((file) => {
    const absPath = join(absDir, file);
    const pkg = readTarballPackageJson(absPath);
    return { name: pkg.name, version: pkg.version, spec: absPath };
  });
}

function collectFromDistDirs(includeEngineManifests) {
  const targets = [];

  if (includeEngineManifests && existsSync(DIST_ENGINE_DIR)) {
    for (const entry of readdirSync(DIST_ENGINE_DIR).sort()) {
      const pkgDir = resolve(DIST_ENGINE_DIR, entry);
      const manifestPath = resolve(pkgDir, 'package.json');
      if (!existsSync(manifestPath)) continue;
      const pkg = JSON.parse(readFileSync(manifestPath, 'utf8'));
      const binDir = resolve(pkgDir, 'bin');
      const hasBinaries = existsSync(binDir) && readdirSync(binDir).length > 0;
      if (!hasBinaries) {
        process.stdout.write(
          `[Bootstrap Notice] Skipping '${pkg.name}' (manifest only, no native binary inside '${binDir}'). Use '--tarball-dir=<dir>' with release .tgz files to bootstrap engine packages.\n`,
        );
        continue;
      }
      targets.push({ name: pkg.name, version: pkg.version, spec: pkgDir });
    }
  }

  for (const dir of PACKAGE_DIRS_ORDER) {
    const pkgDir = resolve(DIST_PACKAGES_DIR, dir);
    const manifestPath = resolve(pkgDir, 'package.json');
    if (!existsSync(manifestPath)) {
      throw new Error(
        `Missing compiled package '${manifestPath}'. Run 'pnpm run build:packages' first.`,
      );
    }
    const pkg = JSON.parse(readFileSync(manifestPath, 'utf8'));
    targets.push({ name: pkg.name, version: pkg.version, spec: pkgDir });
  }

  return targets;
}

function isVersionAlreadyPublished(pkgName, version) {
  try {
    const remoteVersion = runNpm(['view', `${pkgName}@${version}`, 'version'], {
      capture: true,
    }).trim();
    return remoteVersion === version;
  } catch {
    return false;
  }
}

function printTrustedPublisherInstructions(targets) {
  const uniqueNames = [...new Set(targets.map((t) => t.name))];
  process.stdout.write(
    [
      '',
      '=== Next Step: Configure npm OIDC Trusted Publishing (Zero Tokens Needed Afterward) ===',
      '1. Configure Trusted Publisher on each published @ng-desktop/* package settings page:',
      ...uniqueNames.map((name) => `   - https://www.npmjs.com/package/${name}/access`),
      '',
      '   Settings for every package:',
      '     • Publisher:          GitHub Actions',
      '     • Organization/Owner: Gaspar1992',
      '     • Repository:         ng-desktop-engine',
      '     • Workflow filename:  release.yml',
      '     • Environment name:   npm-publish',
      '',
      '2. Enable automatic tokenless OIDC publishing in GitHub Actions:',
      '     gh variable set NPM_TRUSTED_PUBLISHING --body true',
      '',
      '3. Optional (recommended after first OIDC publish succeeds):',
      '     In each package settings page, choose "Require two-factor authentication and disallow tokens".',
      '',
    ].join('\n'),
  );
}

function main() {
  const args = process.argv.slice(2);
  const execute = args.includes('--execute');
  const includeEngineManifests = args.includes('--include-engine-manifests');
  const tarballDir = getFlagValue(args, '--tarball-dir');
  const otp = getFlagValue(args, '--otp');

  const targets = tarballDir
    ? collectFromTarballDir(tarballDir)
    : collectFromDistDirs(includeEngineManifests);

  process.stdout.write(
    `[Bootstrap] Mode: ${execute ? 'EXECUTE (live npm publish)' : 'DRY-RUN (pass --execute to publish)'}\n`,
  );

  if (execute) {
    try {
      const user = runNpm(['whoami'], { capture: true }).trim();
      process.stdout.write(`[Bootstrap] Authenticated to npm as '${user}'.\n`);
    } catch {
      throw new Error(
        "Not logged into npm CLI. Run 'npm login' (with 2FA/passkey) before '--execute'.",
      );
    }
  }

  for (const target of targets) {
    if (isVersionAlreadyPublished(target.name, target.version)) {
      process.stdout.write(
        `[Bootstrap] ${target.name}@${target.version} already exists on npmjs.org — skipping.\n`,
      );
      continue;
    }

    const publishArgs = ['publish', target.spec, '--access', 'public', '--provenance=false'];
    if (otp) {
      publishArgs.push(`--otp=${otp}`);
    }
    if (!execute) {
      publishArgs.push('--dry-run');
    }
    process.stdout.write(
      `[Bootstrap] ${execute ? 'Publishing' : 'Dry-run'} ${target.name}@${target.version}...\n`,
    );
    runNpm(publishArgs);
  }

  printTrustedPublisherInstructions(targets);
}

try {
  main();
} catch (err) {
  process.stderr.write(`[Bootstrap Error] ${err instanceof Error ? err.message : String(err)}\n`);
  process.exitCode = 1;
}
