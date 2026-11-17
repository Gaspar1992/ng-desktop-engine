import {
  createNativeTargetConfig,
  executeNativeBuilder,
  parseTemplateHtml,
  resolveNativeBuilderConfig,
} from '@ng-native/builders';

describe('Phase 6 — @ng-native/builders & Distribution Orchestration', () => {
  it('generates minimal angular.json architect.native config with OS/CPU defaults', () => {
    const cfg = createNativeTargetConfig({
      os: 'linux',
      arch: 'x86_64',
    });

    expect(cfg.builder).toBe('@ng-native/builders:application');
    expect(cfg.options.os).toBe('linux');
    expect(cfg.options.arch).toBe('x86_64');

    const resolvedWin = resolveNativeBuilderConfig('my-win-app', {
      os: 'windows',
      arch: 'aarch64',
      cpu: 'neon',
    });
    expect(resolvedWin.targetPlatform).toBe('windows-winui');
    expect(resolvedWin.jsEngine).toBe('quickjs');
    expect(resolvedWin.rustTargetTriple).toBe('aarch64-pc-windows-msvc');
    expect(resolvedWin.outputPath).toBe('dist/my-win-app');

    const resolvedMac = resolveNativeBuilderConfig('my-mac-app', {
      os: 'macos',
      arch: 'universal',
    });
    expect(resolvedMac.targetPlatform).toBe('macos-appkit');
    expect(resolvedMac.jsEngine).toBe('jsc');
    expect(resolvedMac.rustTargetTriple).toBe('universal-apple-darwin');
  });

  it('executes builder pipeline and produces .deb, AppImage, and Flatpak artifact paths', () => {
    const result = executeNativeBuilder('ng-linux-counter', {
      targetPlatform: 'linux-gtk4',
      jsEngine: 'jsc',
      outputPath: 'dist/ng-linux-counter/',
    });

    expect(result.success).toBe(true);
    expect(result.bundlePath).toBe('dist/ng-linux-counter/main.ngb');
    expect(result.packageArtifacts).toContain('dist/ng-linux-counter/packages/deb/DEBIAN/control');
    expect(result.packageArtifacts).toContain('dist/ng-linux-counter/packages/appimage/AppRun');
    expect(result.packageArtifacts).toContain(
      'dist/ng-linux-counter/packages/flatpak/ng-linux-counter.yml',
    );
  });

  it('compiles Angular 22 templates with @angular/compiler AST including @if blocks and reactive bindings', () => {
    const template = `
      <ng-window id="win" title="{{ windowTitle() }}">
        @if (showBanner) {
          <ng-label id="lbl">Count: {{ count() }}</ng-label>
          <ng-button id="btn" (click)="increment()">+1</ng-button>
        }
      </ng-window>
    `;

    const roots = parseTemplateHtml(
      template,
      { windowTitle: 'AST Demo', showBanner: 'true', count: '5' },
      { count: 5 },
    );

    expect(roots.length).toBe(1);
    expect(roots[0].id).toBe('win');
    expect(roots[0].title).toBe('AST Demo');
    expect(roots[0].children[0].isComment).toBe(true);
    expect(roots[0].children[0].text).toBe('@if(showBanner)');
    expect(roots[0].children[1].id).toBe('lbl');
    expect(roots[0].children[1].reactiveBinding).toEqual({
      signalName: 'count',
      template: 'Count: {}',
    });
    expect(roots[0].children[2].events).toEqual({ click: 'increment' });
  });
});
