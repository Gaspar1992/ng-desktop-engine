import { execFileSync } from 'node:child_process';
import {
  copyFileSync,
  cpSync,
  existsSync,
  mkdirSync,
  readdirSync,
  readFileSync,
  rmSync,
  statSync,
  writeFileSync,
} from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT_DIR = resolve(__dirname, '..');
const DIST_PACKAGES_DIR = resolve(ROOT_DIR, 'dist/packages');
const TMP_TSC_DIR = resolve(ROOT_DIR, 'out-tsc/packages');

const PACKAGE_ORDER = [
  { dir: 'platform-native', compiler: 'ngc' },
  { dir: 'forms-native', compiler: 'ngc' },
  { dir: 'router-native', compiler: 'ngc' },
  { dir: 'http-native', compiler: 'ngc' },
  { dir: 'desktop-native', compiler: 'ngc' },
  { dir: 'builders-native', compiler: 'tsc' },
];

function rewriteRelativeEsmImports(dirPath) {
  for (const entry of readdirSync(dirPath)) {
    const full = resolve(dirPath, entry);
    const stat = statSync(full);
    if (stat.isDirectory()) {
      rewriteRelativeEsmImports(full);
      continue;
    }
    if (!entry.endsWith('.js')) continue;

    const original = readFileSync(full, 'utf8');
    const rewritten = original.replace(
      /(\b(?:from|import)\s*['"])(\.\.?\/[^'"]+)(['"])/g,
      (match, prefix, spec, suffix) => {
        if (
          spec.endsWith('.js') ||
          spec.endsWith('.mjs') ||
          spec.endsWith('.cjs') ||
          spec.endsWith('.json')
        ) {
          return match;
        }
        return `${prefix}${spec}.js${suffix}`;
      },
    );
    if (rewritten !== original) {
      writeFileSync(full, rewritten, 'utf8');
    }
  }
}

function resolveTargetVersion(defaultVersion) {
  const prefix = '--version=';
  for (let i = 0; i < process.argv.length; i += 1) {
    const arg = process.argv[i];
    if (arg.startsWith(prefix)) {
      const val = arg.slice(prefix.length).trim();
      if (val) return val;
    } else if (arg === '--version' && i + 1 < process.argv.length) {
      const val = process.argv[i + 1].trim();
      if (val) return val;
    }
  }
  const envVer = process.env['NG_DESKTOP_APP_VERSION']?.trim();
  return envVer || defaultVersion;
}

function buildSinglePackage(pkgConfig, builtPackages) {
  const pkgSrcRoot = resolve(ROOT_DIR, 'packages', pkgConfig.dir);
  const pkgOutDir = resolve(DIST_PACKAGES_DIR, pkgConfig.dir);
  const fesmOutDir = resolve(pkgOutDir, 'fesm2022');
  const tscWorkDir = resolve(TMP_TSC_DIR, pkgConfig.dir);

  rmSync(pkgOutDir, { recursive: true, force: true });
  rmSync(tscWorkDir, { recursive: true, force: true });
  mkdirSync(fesmOutDir, { recursive: true });
  mkdirSync(tscWorkDir, { recursive: true });

  const pathMappings = {};
  for (const built of builtPackages) {
    pathMappings[built.npmName] = [resolve(built.outDir, 'fesm2022/index.d.ts')];
  }

  const tsconfig = {
    extends: resolve(ROOT_DIR, 'tsconfig.lib.json'),
    compilerOptions: {
      rootDir: resolve(pkgSrcRoot, 'src'),
      outDir: fesmOutDir,
      paths: pathMappings,
      ...(pkgConfig.compiler === 'tsc' ? { types: ['node'] } : {}),
    },
    files: [resolve(pkgSrcRoot, 'src/index.ts')],
  };

  const tsconfigPath = resolve(tscWorkDir, 'tsconfig.json');
  writeFileSync(tsconfigPath, `${JSON.stringify(tsconfig, null, 2)}\n`, 'utf8');

  const binName = pkgConfig.compiler === 'ngc' ? 'ngc' : 'tsc';
  const compilerBin = resolve(ROOT_DIR, 'node_modules/.bin', binName);
  execFileSync(compilerBin, ['-p', tsconfigPath], {
    cwd: ROOT_DIR,
    stdio: 'inherit',
  });

  if (pkgConfig.dir === 'builders-native') {
    const srcDir = resolve(pkgSrcRoot, 'src');
    for (const entry of readdirSync(srcDir)) {
      if (entry.endsWith('.mjs') || entry.endsWith('.d.mts')) {
        copyFileSync(resolve(srcDir, entry), resolve(fesmOutDir, entry));
      }
    }
    for (const extraDir of ['schema', 'bin', 'schematics']) {
      const from = resolve(pkgSrcRoot, extraDir);
      if (existsSync(from)) {
        cpSync(from, resolve(pkgOutDir, extraDir), { recursive: true });
      }
    }
  }

  rewriteRelativeEsmImports(fesmOutDir);

  const rawPkg = JSON.parse(readFileSync(resolve(pkgSrcRoot, 'package.json'), 'utf8'));
  const resolvedVersion = resolveTargetVersion(rawPkg.version);
  const exportsMap = {
    '.': {
      types: './fesm2022/index.d.ts',
      default: './fesm2022/index.js',
    },
    './package.json': {
      default: './package.json',
    },
  };
  if (pkgConfig.dir === 'builders-native') {
    exportsMap['./schema/*'] = { default: './schema/*' };
    exportsMap['./schematics'] = { default: './schematics/collection.json' };
  }

  const apfPkg = {
    name: rawPkg.name,
    version: resolvedVersion,
    description: rawPkg.description ?? `${rawPkg.name} for Angular Desktop Engine`,
    license: 'MIT',
    type: 'module',
    sideEffects: false,
    main: './fesm2022/index.js',
    module: './fesm2022/index.js',
    types: './fesm2022/index.d.ts',
    exports: exportsMap,
    ...(rawPkg.bin ? { bin: rawPkg.bin } : {}),
    ...(rawPkg.schematics ? { schematics: rawPkg.schematics } : {}),
    ...(rawPkg.peerDependencies ? { peerDependencies: rawPkg.peerDependencies } : {}),
    ...(rawPkg.dependencies ? { dependencies: rawPkg.dependencies } : {}),
  };

  writeFileSync(resolve(pkgOutDir, 'package.json'), `${JSON.stringify(apfPkg, null, 2)}\n`, 'utf8');

  const licensePath = resolve(ROOT_DIR, 'LICENSE');
  if (existsSync(licensePath)) {
    copyFileSync(licensePath, resolve(pkgOutDir, 'LICENSE'));
  }

  builtPackages.push({ npmName: rawPkg.name, outDir: pkgOutDir });
  process.stdout.write(`[APF Build] Built ${rawPkg.name}@${resolvedVersion} -> ${pkgOutDir}\n`);
}

function main() {
  mkdirSync(DIST_PACKAGES_DIR, { recursive: true });
  const builtPackages = [];
  for (const pkg of PACKAGE_ORDER) {
    buildSinglePackage(pkg, builtPackages);
  }
  process.stdout.write(
    `[APF Build] Successfully compiled ${builtPackages.length} @ng-native/* packages.\n`,
  );
}

main();
