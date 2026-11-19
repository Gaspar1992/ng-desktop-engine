#!/usr/bin/env node
import { execFileSync } from 'node:child_process';
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readdirSync,
  readFileSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const ROOT_DIR = resolve(fileURLToPath(new URL('..', import.meta.url)));
const DIST_PACKAGES_DIR = resolve(ROOT_DIR, 'dist/packages');

const PACKAGE_DIRS = [
  'platform-native',
  'forms-native',
  'router-native',
  'http-native',
  'desktop-native',
  'builders-native',
];

function assertCondition(cond, message) {
  if (!cond) {
    throw new Error(message);
  }
}

function runNpmPack(cwd, destDir) {
  return execFileSync('npm', ['pack', '--pack-destination', destDir], {
    cwd,
    encoding: 'utf8',
    shell: process.platform === 'win32',
  }).trim();
}

async function main() {
  const workDir = mkdtempSync(join(tmpdir(), 'ng-desktop-pack-test-'));
  const tarballsDir = join(workDir, 'tarballs');
  const unpackDir = join(workDir, 'unpacked');
  const externalWorkspace = join(workDir, 'external-nx-consumer');
  mkdirSync(tarballsDir, { recursive: true });
  mkdirSync(unpackDir, { recursive: true });
  mkdirSync(join(externalWorkspace, 'apps/demo-app/src/app'), { recursive: true });

  try {
    const packedFiles = new Map();
    for (const pkgDir of PACKAGE_DIRS) {
      const pkgPath = resolve(DIST_PACKAGES_DIR, pkgDir);
      assertCondition(
        existsSync(pkgPath),
        `Missing APF output directory '${pkgPath}'. Run 'pnpm run build:packages' first.`,
      );
      const pkgJson = JSON.parse(readFileSync(resolve(pkgPath, 'package.json'), 'utf8'));
      assertCondition(
        pkgJson.name.startsWith('@ng-desktop/'),
        `${pkgDir} unexpected package name '${pkgJson.name}'`,
      );
      const out = runNpmPack(pkgPath, tarballsDir);
      const tgzName = out.split('\n').pop()?.trim();
      assertCondition(Boolean(tgzName), `npm pack returned empty filename for ${pkgDir}`);
      assertCondition(
        tgzName.startsWith('ng-desktop-'),
        `${pkgDir} expected tarball 'ng-desktop-*.tgz', got '${tgzName}'`,
      );
      const tgzAbs = join(tarballsDir, tgzName);
      packedFiles.set(pkgDir, { tgzAbs, pkgJson });

      const contents = execFileSync('tar', ['-tzf', tgzAbs], { encoding: 'utf8' });
      assertCondition(contents.includes('package/fesm2022/index.js'), `${pkgDir} missing index.js`);
      assertCondition(
        contents.includes('package/fesm2022/index.d.ts'),
        `${pkgDir} missing index.d.ts`,
      );
      assertCondition(contents.includes('package/LICENSE'), `${pkgDir} missing LICENSE`);
      assertCondition(contents.includes('package/README.md'), `${pkgDir} missing README.md`);
      if (pkgDir === 'builders-native') {
        assertCondition(
          contents.includes('package/builders.json'),
          'builders-native missing builders.json',
        );
        assertCondition(
          contents.includes('package/bin/ng-desktop.mjs'),
          'builders-native missing bin/ng-desktop.mjs',
        );
        assertCondition(
          contents.includes('package/schematics/collection.json'),
          'builders-native missing schematics/collection.json',
        );
      }
    }

    const buildersInfo = packedFiles.get('builders-native');
    symlinkSync(resolve(ROOT_DIR, 'node_modules'), join(unpackDir, 'node_modules'), 'junction');
    execFileSync('tar', ['-xzf', buildersInfo.tgzAbs, '-C', unpackDir]);
    const unpackedBuildersRoot = join(unpackDir, 'package');
    const buildersMod = await import(
      pathToFileURL(join(unpackedBuildersRoot, 'fesm2022/index.js')).href
    );

    const analysis = buildersMod.analyzeSourcePermissions([
      {
        file: 'app/home.ts',
        content: `import { NativeSqliteService } from '@ng-desktop/desktop'; setClipboard('hi');`,
      },
    ]);
    assertCondition(
      analysis.capabilities.sqlite === true,
      'Packed AST analyzer missed sqlite capability',
    );
    assertCondition(
      analysis.capabilities.clipboard === true,
      'Packed AST analyzer missed clipboard capability',
    );
    assertCondition(
      analysis.capabilities.fullSystemAccess === false,
      'Packed AST analyzer unexpectedly enabled fullSystemAccess',
    );

    writeFileSync(join(externalWorkspace, 'nx.json'), JSON.stringify({ npmScope: 'ext' }), 'utf8');
    writeFileSync(
      join(externalWorkspace, 'apps/demo-app/project.json'),
      JSON.stringify({
        name: 'demo-app',
        projectType: 'application',
        sourceRoot: 'apps/demo-app/src',
        targets: { native: { executor: '@ng-desktop/builders:application', options: {} } },
      }),
      'utf8',
    );
    const nxResolved = buildersMod.resolveWorkspaceProject({ cwd: externalWorkspace });
    assertCondition(
      nxResolved.workspaceKind === 'nx',
      'External packed consumer failed Nx resolution',
    );
    assertCondition(
      nxResolved.projectName === 'demo-app',
      'External packed consumer resolved wrong project',
    );

    const engineRes = buildersMod.resolveEngineBinaryPath({
      env: { NG_DESKTOP_ENGINE_BIN: '/usr/local/bin/ng-native-engine' },
      platform: 'linux',
      arch: 'x64',
    });
    assertCondition(
      engineRes.command === '/usr/local/bin/ng-native-engine',
      'External packed consumer failed resolveEngineBinaryPath env override',
    );

    const cliOut = execFileSync(
      process.execPath,
      [join(unpackedBuildersRoot, 'bin/ng-desktop.mjs'), '--help'],
      {
        encoding: 'utf8',
      },
    );
    assertCondition(cliOut.includes('ng-desktop run'), 'Packed ng-desktop CLI missing run command');

    process.stdout.write(
      `[Pack E2E] Verified ${readdirSync(tarballsDir).length} packed .tgz packages and external consumer execution.\n`,
    );
  } finally {
    rmSync(workDir, { recursive: true, force: true });
  }
}

main().catch((err) => {
  process.stderr.write(`[Pack E2E Error] ${err instanceof Error ? err.message : String(err)}\n`);
  process.exitCode = 1;
});
