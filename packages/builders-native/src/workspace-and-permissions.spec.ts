import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import {
  analyzeSourcePermissions,
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
});
