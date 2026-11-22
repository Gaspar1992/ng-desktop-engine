import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import vm from 'node:vm';
import {
  analyzeSourcePermissions,
  applicationBuilder,
  ARCHITECT_BUILDER_SYMBOL,
  compileBundle,
  devServerBuilder,
  discoverServerRoutes,
  executeNativeBuilder,
  resolveWorkspaceProject,
  runNativeApplicationBuilder,
} from '@ng-desktop/builders';
import { applyNgAddToTree } from '../schematics/ng-add/index.mjs';

describe('Phase 11 — Workspace-Agnostic Resolution & Static Permission Analysis', () => {
  it('resolves Angular CLI angular.json and Nx project.json workspaces dynamically', () => {
    const ngRoot = mkdtempSync(join(tmpdir(), 'ng-desktop-ng-'));
    try {
      mkdirSync(join(ngRoot, 'apps/desktop-notes/src'), { recursive: true });
      writeFileSync(
        join(ngRoot, 'angular.json'),
        JSON.stringify({
          projects: {
            'desktop-notes': {
              projectType: 'application',
              root: 'apps/desktop-notes',
              sourceRoot: 'apps/desktop-notes/src',
              architect: {
                native: {
                  builder: '@ng-desktop/builders:application',
                  options: {
                    os: 'linux',
                    arch: 'x86_64',
                  },
                },
              },
            },
          },
        }),
        'utf8',
      );
      const angularProject = resolveWorkspaceProject({ cwd: ngRoot });
      expect(angularProject.workspaceKind).toBe('angular');
      expect(angularProject.projectName).toBe('desktop-notes');
    } finally {
      rmSync(ngRoot, { recursive: true, force: true });
    }

    const nxRoot = mkdtempSync(join(tmpdir(), 'ng-desktop-nx-'));
    try {
      mkdirSync(join(nxRoot, 'apps/desktop-crm/src'), { recursive: true });
      writeFileSync(join(nxRoot, 'nx.json'), JSON.stringify({ npmScope: 'acme' }), 'utf8');
      writeFileSync(
        join(nxRoot, 'apps/desktop-crm/project.json'),
        JSON.stringify({
          name: 'desktop-crm',
          projectType: 'application',
          sourceRoot: 'apps/desktop-crm/src',
          targets: {
            native: {
              executor: '@ng-desktop/builders:application',
              options: {
                targetPlatform: 'windows-winui',
                outputPath: 'dist/apps/desktop-crm',
                version: '2.1.0',
                permissions: {
                  mode: 'auto',
                  deny: ['clipboard'],
                },
              },
            },
          },
        }),
        'utf8',
      );

      const nxProject = resolveWorkspaceProject({ cwd: nxRoot, projectName: 'desktop-crm' });
      expect(nxProject.workspaceKind).toBe('nx');
      expect(nxProject.projectName).toBe('desktop-crm');
      expect(nxProject.sourceRoot).toBe('apps/desktop-crm/src');
      expect(nxProject.outputPath).toBe('dist/apps/desktop-crm');
      expect(nxProject.version).toBe('2.1.0');
    } finally {
      rmSync(nxRoot, { recursive: true, force: true });
    }
  });

  it('infers exact OS permissions from TypeScript AST and honors strict/allow/deny policy without fullSystemAccess', () => {
    const sampleFiles = [
      {
        file: 'app/services/api.ts',
        content: `
          import { HttpClient } from '@angular/common/http';
          import { NativeSqliteService } from '@ng-desktop/desktop';

          export function syncAll(sql: NativeSqliteService) {
            fetch('https://example.test/api');
            sql.query('SELECT 1');
            openFileDialog('Select Backup', '/tmp/a.json');
            setClipboard('copied');
            sendNotification('Sync', 'Done');
            computeHash('abc');
          }
        `,
      },
      {
        file: 'app/app.html',
        content: `<ng-window (file-drop)="onDrop($event)"></ng-window>`,
      },
    ];

    const autoManifest = analyzeSourcePermissions(sampleFiles, {
      mode: 'auto',
      deny: ['clipboard'],
    });
    expect(autoManifest.mode).toBe('auto');
    expect(autoManifest.capabilities.network).toBe(true);
    expect(autoManifest.capabilities.sqlite).toBe(true);
    expect(autoManifest.capabilities.filesystem.dialogs).toBe(true);
    expect(autoManifest.capabilities.filesystem.drop).toBe(true);
    expect(autoManifest.capabilities.notifications).toBe(true);
    expect(autoManifest.capabilities.computeThreads).toBe(true);
    expect(autoManifest.capabilities.clipboard).toBe(false);
    expect(autoManifest.capabilities.fullSystemAccess).toBe(false);
    expect(autoManifest.warnings).toEqual([]);

    const strictManifest = analyzeSourcePermissions(sampleFiles, {
      mode: 'strict',
      allow: ['network'],
    });
    expect(strictManifest.mode).toBe('strict');
    expect(strictManifest.capabilities.network).toBe(true);
    expect(strictManifest.capabilities.sqlite).toBe(false);
    expect(strictManifest.capabilities.filesystem.dialogs).toBe(false);
    expect(strictManifest.capabilities.fullSystemAccess).toBe(false);
    expect(strictManifest.warnings.length).toBeGreaterThan(0);

    const builderResult = executeNativeBuilder('nx-app', {
      os: 'windows',
      outputPath: 'dist/nx-app',
    });
    expect(builderResult.permissionsManifestPath).toBe('dist/nx-app/permissions.manifest.json');
    expect(builderResult.runtimeConfigPath).toBe('dist/nx-app/ng-desktop.runtime.json');
  });

  it('configures both Angular CLI angular.json and Nx project.json via ng-add schematic helper', () => {
    const createMockTree = (initial: Record<string, string>) => {
      const files = new Map(Object.entries(initial));
      return {
        exists: (path: string) => files.has(path),
        read: (path: string) => files.get(path) ?? null,
        overwrite: (path: string, content: string) => {
          files.set(path, content);
        },
        readJson: (path: string) => JSON.parse(files.get(path) ?? '{}') as Record<string, unknown>,
      };
    };

    const angularTree = createMockTree({
      '/package.json': JSON.stringify({ name: 'demo', dependencies: {} }),
      '/angular.json': JSON.stringify({
        defaultProject: 'desktop-app',
        projects: { 'desktop-app': { architect: {} } },
      }),
    });
    applyNgAddToTree(angularTree, {
      targetPlatform: 'windows-winui',
      permissionsMode: 'strict',
    });
    const updatedAngular = angularTree.readJson('/angular.json') as {
      projects: Record<string, { architect: Record<string, unknown> }>;
    };
    expect(updatedAngular.projects['desktop-app']?.architect['native']).toEqual({
      builder: '@ng-desktop/builders:application',
      options: {
        targetPlatform: 'windows-winui',
        permissions: { mode: 'strict' },
      },
    });

    const nxTree = createMockTree({
      '/package.json': JSON.stringify({ name: 'nx-repo', dependencies: {} }),
      '/apps/billing/project.json': JSON.stringify({ name: 'billing', targets: {} }),
    });
    applyNgAddToTree(nxTree, { project: 'billing', targetPlatform: 'macos-appkit' });
    const updatedNx = nxTree.readJson('/apps/billing/project.json') as {
      targets: Record<string, unknown>;
    };
    expect(updatedNx.targets['native']).toEqual({
      executor: '@ng-desktop/builders:application',
      options: {
        targetPlatform: 'macos-appkit',
        permissions: { mode: 'auto' },
      },
    });
  });

  it('emits app.bundle.js and records entryBundle when Angular browser build output exists', () => {
    const tmpRoot = mkdtempSync(join(tmpdir(), 'ng-desktop-ivy-bundle-'));
    try {
      mkdirSync(join(tmpRoot, 'src/app'), { recursive: true });
      mkdirSync(join(tmpRoot, 'dist/sample-app/browser'), { recursive: true });
      writeFileSync(
        join(tmpRoot, 'angular.json'),
        JSON.stringify({
          projects: {
            'sample-app': {
              projectType: 'application',
              root: '.',
              sourceRoot: 'src',
              architect: {
                native: {
                  builder: '@ng-desktop/builders:application',
                  options: {
                    outputPath: 'dist/sample-app',
                  },
                },
              },
            },
          },
        }),
      );
      writeFileSync(
        join(tmpRoot, 'src/app/app.ts'),
        `import { Component } from '@angular/core';
@Component({ selector: 'app-root', templateUrl: './app.html', styleUrl: './app.css' })
export class App {}`,
      );
      writeFileSync(
        join(tmpRoot, 'src/app/app.html'),
        `<ng-window id="win" title="Sample Ivy"><ng-label id="lbl">Ready</ng-label></ng-window>`,
      );
      writeFileSync(
        join(tmpRoot, 'src/app/app.css'),
        `.badge { background: #1e293b; color: #f8fafc; padding: 8px; border-radius: 6px; }`,
      );
      writeFileSync(
        join(tmpRoot, 'dist/sample-app/browser/polyfills-TEST.js'),
        `globalThis.__POLYFILL_LOADED__ = true;`,
      );
      writeFileSync(
        join(tmpRoot, 'dist/sample-app/browser/main-TEST.js'),
        `console.log('compiled-ivy-main');`,
      );

      const res = compileBundle({ cwd: tmpRoot, projectName: 'sample-app', rebuildIcons: false });
      expect(res.entryBundleFile).toBe(join(tmpRoot, 'dist/sample-app/app.bundle.js'));

      const appBundle = readFileSync(res.entryBundleFile!, 'utf8');
      expect(appBundle).toContain('globalThis.__NG_NATIVE_APP_ID__ = "sample-app";');
      expect(appBundle).toContain(
        'globalThis.__NG_NATIVE__?.registerStyle("badge", "#1e293b", "#f8fafc", 8, 0, "radius=6");',
      );
      expect(appBundle).toMatch(/globalThis\.__POLYFILL_LOADED__\s*=\s*(?:!0|true);/);
      expect(appBundle).toMatch(/console\.log\(["']compiled-ivy-main["']\);/);

      const runtimeCfg = JSON.parse(readFileSync(res.runtimeConfigFile, 'utf8')) as {
        entryBundle?: string;
      };
      expect(runtimeCfg.entryBundle).toBe('app.bundle.js');
    } finally {
      rmSync(tmpRoot, { recursive: true, force: true });
    }
  });

  it('discovers Analog-style server routes, compiles server.bundle.js, and detects context permissions', async () => {
    const tmpRoot = mkdtempSync(join(tmpdir(), 'ng-desktop-server-routes-'));
    try {
      mkdirSync(join(tmpRoot, 'src/app'), { recursive: true });
      mkdirSync(join(tmpRoot, 'src/server/routes/api/favorites'), { recursive: true });
      mkdirSync(join(tmpRoot, 'src/server/routes/api/pokemon'), { recursive: true });
      mkdirSync(join(tmpRoot, 'src/server/routes/api/files'), { recursive: true });

      writeFileSync(
        join(tmpRoot, 'angular.json'),
        JSON.stringify({
          projects: {
            'pokedex-app': {
              projectType: 'application',
              root: '.',
              sourceRoot: 'src',
              architect: {
                native: {
                  builder: '@ng-desktop/builders:application',
                  options: {
                    outputPath: 'dist/pokedex-app',
                  },
                },
              },
            },
          },
        }),
      );
      writeFileSync(
        join(tmpRoot, 'src/app/app.ts'),
        `import { Component } from '@angular/core';
@Component({ selector: 'app-root', templateUrl: './app.html', styleUrl: './app.css' })
export class App {}`,
      );
      writeFileSync(
        join(tmpRoot, 'src/app/app.html'),
        `<ng-window id="win" title="Server Test"></ng-window>`,
      );
      writeFileSync(join(tmpRoot, 'src/app/app.css'), `.panel { padding: 8px; }`);

      writeFileSync(
        join(tmpRoot, 'src/server/routes/api/favorites/index.get.ts'),
        `import { defineNativeEventHandler } from '@ng-desktop/server';
export default defineNativeEventHandler((event) => {
  event.context.db.execute('CREATE TABLE IF NOT EXISTS fav (id TEXT PRIMARY KEY);');
  return { items: event.context.db.query('SELECT * FROM fav;') };
});`,
      );
      writeFileSync(
        join(tmpRoot, 'src/server/routes/api/pokemon/[id].get.ts'),
        `import { defineNativeEventHandler, getRouterParam } from '@ng-desktop/server';
export default defineNativeEventHandler(async (event) => {
  const id = getRouterParam(event, 'id');
  const upstream = await event.context.http.get('https://pokeapi.co/api/v2/pokemon/' + id);
  const path = event.context.fs.resolvePath('cache', id + '.json');
  return { id, status: upstream.status, hash: event.context.compute.hash(String(id)), path };
});`,
      );
      writeFileSync(
        join(tmpRoot, 'src/server/routes/api/files/[...path].post.ts'),
        `import { defineNativeEventHandler, getRouterParam } from '@ng-desktop/server';
export default defineNativeEventHandler((event) => ({
  file: getRouterParam(event, 'path'),
}));`,
      );

      const routes = discoverServerRoutes(join(tmpRoot, 'src'));
      expect(routes.map((r) => `${r.method} ${r.pattern}`)).toEqual([
        'GET /api/favorites',
        'GET /api/pokemon/:id',
        'POST /api/files/*path',
      ]);

      const res = compileBundle({ cwd: tmpRoot, projectName: 'pokedex-app', rebuildIcons: false });
      expect(res.serverBundleFile).toBe(join(tmpRoot, 'dist/pokedex-app/server.bundle.js'));
      expect(res.permissions.capabilities.sqlite).toBe(true);
      expect(res.permissions.capabilities.filesystem.xdg).toBe(true);
      expect(res.permissions.capabilities.computeThreads).toBe(true);
      expect(res.permissions.capabilities.network).toBe(true);

      const runtimeCfg = JSON.parse(readFileSync(res.runtimeConfigFile, 'utf8')) as {
        serverBundle?: string;
      };
      expect(runtimeCfg.serverBundle).toBe('server.bundle.js');

      const bundleCode = readFileSync(res.serverBundleFile!, 'utf8');
      const sandbox: Record<string, unknown> = { console, URL, URLSearchParams };
      sandbox['globalThis'] = sandbox;
      vm.runInNewContext(bundleCode, sandbox);
      const router = sandbox['__NG_NATIVE_SERVER_ROUTER__'] as {
        handle: (req: { method: string; url: string }) => Promise<{ status: number; body: string }>;
      };
      const fileRes = await router.handle({
        method: 'POST',
        url: '/api/files/exports/team.json',
      });
      expect(fileRes.status).toBe(200);
      expect(JSON.parse(fileRes.body)).toEqual({ file: 'exports/team.json' });
    } finally {
      rmSync(tmpRoot, { recursive: true, force: true });
    }
  });

  it('bundles lazy dynamic import chunks inside app.bundle.js via esbuild', async () => {
    const tmpRoot = mkdtempSync(join(tmpdir(), 'ng-desktop-lazy-bundle-'));
    try {
      mkdirSync(join(tmpRoot, 'src/app'), { recursive: true });
      mkdirSync(join(tmpRoot, 'dist/lazy-app/browser'), { recursive: true });
      writeFileSync(
        join(tmpRoot, 'angular.json'),
        JSON.stringify({
          projects: {
            'lazy-app': {
              projectType: 'application',
              root: '.',
              sourceRoot: 'src',
              architect: {
                native: {
                  builder: '@ng-desktop/builders:application',
                  options: { outputPath: 'dist/lazy-app' },
                },
              },
            },
          },
        }),
      );
      writeFileSync(join(tmpRoot, 'src/app/app.ts'), 'export class App {}');
      writeFileSync(
        join(tmpRoot, 'src/app/app.html'),
        '<ng-window id="win" title="Lazy"><ng-label id="lbl">Ok</ng-label></ng-window>',
      );
      writeFileSync(join(tmpRoot, 'src/app/app.css'), '');
      writeFileSync(
        join(tmpRoot, 'dist/lazy-app/browser/chunk-LAZY.js'),
        "export function lazyGreeting() { return 'hello-lazy-chunk'; }",
      );
      writeFileSync(
        join(tmpRoot, 'dist/lazy-app/browser/main-TEST.js'),
        "import('./chunk-LAZY.js').then((m) => { globalThis.__LAZY_RESULT__ = m.lazyGreeting(); });",
      );

      const res = compileBundle({ cwd: tmpRoot, projectName: 'lazy-app', rebuildIcons: false });
      const appBundle = readFileSync(res.entryBundleFile!, 'utf8');
      expect(appBundle).toContain('hello-lazy-chunk');

      const sandbox: Record<string, unknown> = { console, Promise };
      sandbox['globalThis'] = sandbox;
      vm.runInNewContext(appBundle, sandbox);
      await Promise.resolve();
      await Promise.resolve();
      expect(sandbox['__LAZY_RESULT__']).toBe('hello-lazy-chunk');
    } finally {
      rmSync(tmpRoot, { recursive: true, force: true });
    }
  });

  it('tree-shakes InMemory fallback stores in production app.bundle.js via __NG_DESKTOP_PROD_BUNDLE__', () => {
    const tmpRoot = mkdtempSync(join(tmpdir(), 'ng-desktop-treeshake-'));
    try {
      mkdirSync(join(tmpRoot, 'src/app'), { recursive: true });
      mkdirSync(join(tmpRoot, 'dist/prod-app/browser'), { recursive: true });
      writeFileSync(
        join(tmpRoot, 'angular.json'),
        JSON.stringify({
          projects: {
            'prod-app': {
              projectType: 'application',
              root: '.',
              sourceRoot: 'src',
              architect: {
                native: {
                  builder: '@ng-desktop/builders:application',
                  options: { outputPath: 'dist/prod-app' },
                },
              },
            },
          },
        }),
      );
      writeFileSync(join(tmpRoot, 'src/app/app.ts'), 'export class App {}');
      writeFileSync(
        join(tmpRoot, 'src/app/app.html'),
        '<ng-window id="win" title="Prod"><ng-label id="lbl">Ok</ng-label></ng-window>',
      );
      writeFileSync(join(tmpRoot, 'src/app/app.css'), '');
      const bridgeTsPath = join(process.cwd(), 'packages/platform-native/src/bridge.ts');
      writeFileSync(
        join(tmpRoot, 'dist/prod-app/browser/main-TEST.js'),
        `import { ensureNgNativeBridge } from ${JSON.stringify(bridgeTsPath)};
globalThis.__CALL_ENSURE__ = ensureNgNativeBridge;`,
      );

      const res = compileBundle({ cwd: tmpRoot, projectName: 'prod-app', rebuildIcons: false });
      const appBundle = readFileSync(res.entryBundleFile!, 'utf8');
      expect(appBundle).not.toContain('InMemorySqliteStore');
      expect(appBundle).not.toContain('InMemoryFsStore');

      const sandboxNoBridge: Record<string, unknown> = { console };
      sandboxNoBridge['globalThis'] = sandboxNoBridge;
      vm.runInNewContext(appBundle, sandboxNoBridge);
      expect(() => (sandboxNoBridge['__CALL_ENSURE__'] as () => unknown)()).toThrow(
        'Native __NG_NATIVE__ bridge is required in production bundles.',
      );

      const mockBridge = { registerStyle: () => {} };
      const sandboxWithBridge: Record<string, unknown> = {
        console,
        __NG_NATIVE__: mockBridge,
      };
      sandboxWithBridge['globalThis'] = sandboxWithBridge;
      vm.runInNewContext(appBundle, sandboxWithBridge);
      expect((sandboxWithBridge['__CALL_ENSURE__'] as () => unknown)()).toBe(mockBridge);
    } finally {
      rmSync(tmpRoot, { recursive: true, force: true });
    }
  });

  it('exports @angular-devkit/architect builders with incremental hmrDelta tracking', async () => {
    expect(applicationBuilder[ARCHITECT_BUILDER_SYMBOL]).toBe(true);
    expect(devServerBuilder[ARCHITECT_BUILDER_SYMBOL]).toBe(true);

    const tmpRoot = mkdtempSync(join(tmpdir(), 'ng-desktop-architect-'));
    try {
      mkdirSync(join(tmpRoot, 'src/app'), { recursive: true });
      mkdirSync(join(tmpRoot, 'src/server/routes/api'), { recursive: true });
      writeFileSync(
        join(tmpRoot, 'angular.json'),
        JSON.stringify({
          projects: {
            'architect-app': {
              projectType: 'application',
              root: '.',
              sourceRoot: 'src',
              architect: {
                native: {
                  builder: '@ng-desktop/builders:application',
                  options: { outputPath: 'dist/architect-app' },
                },
              },
            },
          },
        }),
      );
      writeFileSync(
        join(tmpRoot, 'src/app/app.ts'),
        `import { Component } from '@angular/core';
@Component({ selector: 'app-root', templateUrl: './app.html', styleUrl: './app.css' })
export class App {}`,
      );
      writeFileSync(
        join(tmpRoot, 'src/app/app.html'),
        `<ng-window id="win" title="Architect App"><ng-label id="lbl">Ready</ng-label></ng-window>`,
      );
      writeFileSync(join(tmpRoot, 'src/app/app.css'), `.panel { padding: 8px; }`);
      writeFileSync(
        join(tmpRoot, 'src/server/routes/api/health.get.ts'),
        `import { defineNativeEventHandler } from '@ng-desktop/server';
export default defineNativeEventHandler(() => ({ ok: true, rev: 1 }));`,
      );

      const logs: string[] = [];
      const ctx = {
        workspaceRoot: tmpRoot,
        target: { project: 'architect-app', target: 'native' },
        logger: {
          info: (msg: string) => logs.push(msg),
        },
      };

      const first = await runNativeApplicationBuilder({ rebuildIcons: false }, ctx);
      expect(first.success).toBe(true);
      expect(first.projectName).toBe('architect-app');
      expect(first.hmrDelta.ngbChanged).toBe(true);
      expect(first.hmrDelta.serverBundleChanged).toBe(true);
      expect(logs[0]).toContain('[NgDesktop Architect] Built architect-app');

      const second = await runNativeApplicationBuilder({ rebuildIcons: false }, ctx);
      expect(second.hmrDelta.ngbChanged).toBe(false);
      expect(second.hmrDelta.serverBundleChanged).toBe(false);

      writeFileSync(
        join(tmpRoot, 'src/server/routes/api/health.get.ts'),
        `import { defineNativeEventHandler } from '@ng-desktop/server';
export default defineNativeEventHandler(() => ({ ok: true, rev: 2 }));`,
      );
      const third = await runNativeApplicationBuilder({ rebuildIcons: false }, ctx);
      expect(third.hmrDelta.ngbChanged).toBe(false);
      expect(third.hmrDelta.serverBundleChanged).toBe(true);
    } finally {
      rmSync(tmpRoot, { recursive: true, force: true });
    }
  });
});
