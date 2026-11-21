import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, readdirSync, statSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

function getModuleDir() {
  if (typeof import.meta.url === 'string' && import.meta.url.startsWith('file:')) {
    return dirname(fileURLToPath(import.meta.url));
  }
  return resolve(process.cwd(), 'packages/builders-native/src');
}

const HTTP_METHOD_SUFFIX = /\.(get|post|put|patch|delete|head|options)$/i;

function segmentWeight(seg) {
  if (seg.startsWith('*')) return 1;
  if (seg.startsWith(':')) return 2;
  return 3;
}

function routeTier(segs) {
  if (segs.some((seg) => seg.startsWith('*'))) return 1;
  if (segs.some((seg) => seg.startsWith(':'))) return 2;
  return 3;
}

function compareDiscoveredRoutes(a, b) {
  const segsA = a.pattern.split('/').filter(Boolean);
  const segsB = b.pattern.split('/').filter(Boolean);
  const tierDiff = routeTier(segsB) - routeTier(segsA);
  if (tierDiff !== 0) return tierDiff;
  const maxLen = Math.max(segsA.length, segsB.length);
  for (let i = 0; i < maxLen; i += 1) {
    const wA = segsA[i] ? segmentWeight(segsA[i]) : 0;
    const wB = segsB[i] ? segmentWeight(segsB[i]) : 0;
    if (wA !== wB) return wB - wA;
  }
  const exactA = a.method !== '*';
  const exactB = b.method !== '*';
  if (exactA !== exactB) return exactA ? -1 : 1;
  return a.pattern.localeCompare(b.pattern) || a.method.localeCompare(b.method);
}

function mapFileToRoute(absFile, routesRootAbs) {
  const rel = absFile.slice(routesRootAbs.length).replaceAll('\\', '/').replace(/^\/+/, '');
  const withoutExt = rel.replace(/\.ts$/i, '');

  let method = '*';
  let routeStem = withoutExt;
  const methodMatch = withoutExt.match(HTTP_METHOD_SUFFIX);
  if (methodMatch) {
    method = methodMatch[1].toUpperCase();
    routeStem = withoutExt.slice(0, -methodMatch[0].length);
  }

  if (routeStem === 'index') {
    routeStem = '';
  } else if (routeStem.endsWith('/index')) {
    routeStem = routeStem.slice(0, -'/index'.length);
  }

  const mappedSegments = routeStem
    .split('/')
    .filter(Boolean)
    .map((seg) => {
      const catchAll = seg.match(/^\[\.\.\.([^\]]+)\]$/);
      if (catchAll) return `*${catchAll[1]}`;
      const dynamicParam = seg.match(/^\[([^\]]+)\]$/);
      if (dynamicParam) return `:${dynamicParam[1]}`;
      return seg;
    });

  return {
    method,
    pattern: `/${mappedSegments.join('/')}`,
    filePath: absFile,
  };
}

function collectRouteFiles(dirPath, out = []) {
  if (!existsSync(dirPath)) return out;
  for (const entry of readdirSync(dirPath)) {
    const full = resolve(dirPath, entry);
    const stat = statSync(full);
    if (stat.isDirectory()) {
      collectRouteFiles(full, out);
    } else if (
      entry.endsWith('.ts') &&
      !entry.endsWith('.spec.ts') &&
      !entry.endsWith('.test.ts') &&
      !entry.endsWith('.d.ts')
    ) {
      out.push(full);
    }
  }
  return out;
}

/**
 * Discovers AnalogJS-style file-based routes under `<sourceRoot>/server/routes/`.
 */
export function discoverServerRoutes(sourceRootAbs) {
  const routesRootAbs = resolve(sourceRootAbs, 'server/routes');
  if (!existsSync(routesRootAbs)) {
    return [];
  }
  const files = collectRouteFiles(routesRootAbs);
  return files.map((file) => mapFileToRoute(file, routesRootAbs)).sort(compareDiscoveredRoutes);
}

function findPnpmEsbuild(baseDir) {
  const pnpmDir = resolve(baseDir, 'node_modules/.pnpm');
  if (!existsSync(pnpmDir)) return null;
  const esbuildPkg = readdirSync(pnpmDir).find((name) => name.startsWith('esbuild@'));
  if (!esbuildPkg) return null;
  const candidate = resolve(pnpmDir, esbuildPkg, 'node_modules/esbuild');
  return existsSync(candidate) ? candidate : null;
}

function resolveEsbuildPath(workspaceRoot) {
  const moduleDir = getModuleDir();
  const roots = [workspaceRoot, resolve(moduleDir, '../../..'), process.cwd()];
  for (const root of roots) {
    try {
      const req = createRequire(resolve(root, 'package.json'));
      return req.resolve('esbuild');
    } catch {
      const pnpmPath = findPnpmEsbuild(root);
      if (pnpmPath) {
        return pnpmPath;
      }
    }
  }
  throw new Error("Cannot locate 'esbuild' to compile server.bundle.js");
}

export function runEsbuildBuild({
  workspaceRoot,
  resolveDir = workspaceRoot,
  entrySource,
  loader = 'ts',
  alias,
  define,
  minifySyntax = false,
  banner,
}) {
  const esbuildPath = resolveEsbuildPath(workspaceRoot);
  const buildOptions = {
    stdin: {
      contents: entrySource,
      resolveDir,
      loader,
    },
    bundle: true,
    format: 'iife',
    platform: 'browser',
    target: 'es2022',
    write: false,
    mainFields: ['fesm2022', 'module', 'main'],
    ...(alias ? { alias } : {}),
    ...(define ? { define } : {}),
    ...(minifySyntax ? { minifySyntax: true } : {}),
    ...(banner ? { banner } : {}),
  };

  if (typeof window === 'undefined') {
    const req = createRequire(resolve(workspaceRoot, 'package.json'));
    const esbuild = req(esbuildPath);
    const built = esbuild.buildSync(buildOptions);
    return built.outputFiles[0].text;
  }

  // Vitest jsdom realms override Uint8Array/TextEncoder prototypes; isolate esbuild in Node.
  const runner = [
    "const fs = require('node:fs');",
    'const esbuild = require(process.argv[1]);',
    "const buildOptions = JSON.parse(fs.readFileSync(0, 'utf8'));",
    'const built = esbuild.buildSync(buildOptions);',
    'process.stdout.write(built.outputFiles[0].text);',
  ].join('\n');

  return execFileSync(process.execPath, ['-e', runner, esbuildPath], {
    input: JSON.stringify(buildOptions),
    encoding: 'utf8',
    maxBuffer: 32 * 1024 * 1024,
  });
}

/**
 * Bundles discovered `src/server/routes/**` handlers into `<outDir>/server.bundle.js`.
 */
export function compileServerBundle({ workspaceRoot, sourceRootAbs, outDir, projectName }) {
  const routes = discoverServerRoutes(sourceRootAbs);
  if (routes.length === 0) {
    return undefined;
  }

  mkdirSync(outDir, { recursive: true });

  const imports = [];
  const entries = [];
  routes.forEach((route, idx) => {
    const varName = `routeHandler${idx}`;
    imports.push(`import ${varName} from ${JSON.stringify(route.filePath)};`);
    entries.push(
      `  { method: ${JSON.stringify(route.method)}, pattern: ${JSON.stringify(route.pattern)}, handler: ${varName} },`,
    );
  });

  const entrySource = [
    `import { registerNativeServerApp } from '@ng-desktop/server';`,
    ...imports,
    `registerNativeServerApp([`,
    ...entries,
    `]);`,
  ].join('\n');

  const localServerEntry = resolve(workspaceRoot, 'packages/server-native/src/index.ts');
  const repoServerEntry = resolve(getModuleDir(), '../..', 'server-native/src/index.ts');
  const cwdServerEntry = resolve(process.cwd(), 'packages/server-native/src/index.ts');
  const alias = {};
  if (existsSync(localServerEntry)) {
    alias['@ng-desktop/server'] = localServerEntry;
  } else if (existsSync(repoServerEntry)) {
    alias['@ng-desktop/server'] = repoServerEntry;
  } else if (existsSync(cwdServerEntry)) {
    alias['@ng-desktop/server'] = cwdServerEntry;
  }

  const outputText = runEsbuildBuild({ workspaceRoot, entrySource, alias });
  const preamble = `globalThis.__NG_NATIVE_APP_ID__ = ${JSON.stringify(projectName)};\n`;
  const serverBundleFile = resolve(outDir, 'server.bundle.js');
  writeFileSync(serverBundleFile, preamble + outputText, 'utf8');
  return serverBundleFile;
}
