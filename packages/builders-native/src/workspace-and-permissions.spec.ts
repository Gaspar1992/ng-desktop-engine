import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import {
  analyzeSourcePermissions,
  compileBundle,
  executeNativeBuilder,
  resolveWorkspaceProject,
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
      expect(appBundle).toContain('globalThis.__POLYFILL_LOADED__ = true;');
      expect(appBundle).toContain("console.log('compiled-ivy-main');");

      const runtimeCfg = JSON.parse(readFileSync(res.runtimeConfigFile, 'utf8')) as {
        entryBundle?: string;
      };
      expect(runtimeCfg.entryBundle).toBe('app.bundle.js');
    } finally {
      rmSync(tmpRoot, { recursive: true, force: true });
    }
  });
});
