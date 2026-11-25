export interface PackageSpec {
  readonly id: string;
  readonly name: string;
  readonly version: string;
  readonly kind: 'library' | 'binary';
  readonly summary: string;
  readonly exportsList: readonly string[];
  readonly installCmd: string;
  readonly snippet: string;
}

export const PACKAGES_CATALOG: readonly PackageSpec[] = [
  {
    id: 'platform',
    name: '@ng-desktop/platform',
    version: '0.6.0',
    kind: 'library',
    summary:
      'Core Angular 22 Renderer2 engine bridge, 29 widget directives, CSS Flexbox/Grid runtime compiler, and O(log n) Fenwick virtual list/grid.',
    exportsList: [
      'bootstrapNativeApplication',
      'provideNativePlatform',
      'NATIVE_UI_DIRECTIVES',
      'NativeVirtualListComponent',
      'NativeVirtualGridComponent',
      'parseRuntimeStyles',
    ],
    installCmd: 'pnpm add @ng-desktop/platform',
    snippet: `import { bootstrapNativeApplication, provideNativePlatform } from '@ng-desktop/platform';

bootstrapNativeApplication(App, {
  providers: [provideNativePlatform()],
});`,
  },
  {
    id: 'builders',
    name: '@ng-desktop/builders',
    version: '0.6.0',
    kind: 'library',
    summary:
      'Angular CLI `ng add` schematic, Ahead-of-Time `.ngb` binary compiler (AST + scoped CSS), HMR dev server, permission analyzer, and OS packager.',
    exportsList: [
      'ng add @ng-desktop/builders',
      'ng-desktop dev / build / package',
      'buildBundle',
      'compileCssToNativeRules',
      'analyzePermissions',
    ],
    installCmd: 'ng add @ng-desktop/builders',
    snippet: `ng add @ng-desktop/builders
pnpm exec ng-desktop dev
pnpm exec ng-desktop package --target deb`,
  },
  {
    id: 'forms',
    name: '@ng-desktop/forms',
    version: '0.6.0',
    kind: 'library',
    summary:
      'First-class Angular Signal Forms (`[formField]`) integration plus reactive/template-driven `ControlValueAccessor` directives for all native inputs.',
    exportsList: [
      'NATIVE_FORM_CONTROL_DIRECTIVES',
      'NativeSignalFieldDirective',
      'provideNativeForms',
      'NativeEntryValueAccessor',
      'NativeSwitchValueAccessor',
    ],
    installCmd: 'pnpm add @ng-desktop/forms',
    snippet: `import { form, required } from '@angular/forms/signals';
import { NATIVE_FORM_CONTROL_DIRECTIVES } from '@ng-desktop/forms';`,
  },
  {
    id: 'router',
    name: '@ng-desktop/router',
    version: '0.6.0',
    kind: 'library',
    summary:
      'Memory-backed hash/location navigation (`provideNativeRouter`) paired with `NativeStackRouteReuseStrategy` preserving scroll and view state.',
    exportsList: ['provideNativeRouter', 'NativeStackRouteReuseStrategy', 'NativePlatformLocation'],
    installCmd: 'pnpm add @ng-desktop/router',
    snippet: `import { provideNativeRouter } from '@ng-desktop/router';

export const appConfig = {
  providers: [provideNativeRouter(routes)],
};`,
  },
  {
    id: 'http',
    name: '@ng-desktop/http',
    version: '0.6.0',
    kind: 'library',
    summary:
      'Zero-CORS Rust `reqwest`/`rustls` HTTP client backend (`provideNativeHttp`), JSON interceptors, and Signal-first `createNativeHttpResource`.',
    exportsList: ['provideNativeHttp', 'NativeHttpBackend', 'createNativeHttpResource'],
    installCmd: 'pnpm add @ng-desktop/http',
    snippet: `readonly pokemon = createNativeHttpResource(
  () => \`/api/pokemon/\${this.id()}\`,
  { defaultValue: null },
);`,
  },
  {
    id: 'desktop',
    name: '@ng-desktop/desktop',
    version: '0.6.0',
    kind: 'library',
    summary:
      'Capability-gated desktop APIs: sandboxed filesystem I/O, local SQLite, OS dialogs, system tray menus, clipboard, shortcuts, and drag-and-drop.',
    exportsList: [
      'NativeDesktopService',
      'NativeFsService',
      'NativeSqliteService',
      'NativeComputeService',
      'NativeDropTargetDirective',
    ],
    installCmd: 'pnpm add @ng-desktop/desktop',
    snippet: `const desktop = inject(NativeDesktopService);
const folder = await desktop.openDirectoryDialog('Select Export Target');`,
  },
  {
    id: 'server',
    name: '@ng-desktop/server',
    version: '0.6.0',
    kind: 'library',
    summary:
      'Local-first Nitro/h3-inspired in-process server routes backed directly by Rust SQLite, eliminating separate Node daemons in desktop apps.',
    exportsList: [
      'provideNativeServerRoutes',
      'defineServerHandler',
      'ServerRouteEvent',
      'SqliteServerAdapter',
    ],
    installCmd: 'pnpm add @ng-desktop/server',
    snippet: `provideNativeServerRoutes([
  { path: '/api/pokemon', handler: () => db.query('SELECT * FROM pokemon') },
]);`,
  },
  {
    id: 'engine-linux-x64',
    name: '@ng-desktop/engine-linux-x64',
    version: '0.6.0',
    kind: 'binary',
    summary: 'Prebuilt Linux x86_64 GTK4 + Libadwaita + QuickJS-NG / JavaScriptCore engine binary.',
    exportsList: ['bin/ng-native-engine (ELF x86_64)'],
    installCmd: 'Installed automatically as optionalDependency of @ng-desktop/builders',
    snippet: `"optionalDependencies": { "@ng-desktop/engine-linux-x64": "0.6.0" }`,
  },
  {
    id: 'engine-linux-arm64',
    name: '@ng-desktop/engine-linux-arm64',
    version: '0.6.0',
    kind: 'binary',
    summary: 'Prebuilt Linux aarch64 GTK4 + Libadwaita engine binary for ARM workstations & SBCs.',
    exportsList: ['bin/ng-native-engine (ELF aarch64)'],
    installCmd: 'Installed automatically as optionalDependency of @ng-desktop/builders',
    snippet: `"optionalDependencies": { "@ng-desktop/engine-linux-arm64": "0.6.0" }`,
  },
  {
    id: 'engine-win32-x64',
    name: '@ng-desktop/engine-win32-x64',
    version: '0.6.0',
    kind: 'binary',
    summary:
      'Prebuilt Windows 11 x86_64 WinUI 3 + QuickJS-NG engine executable with MSIX packaging.',
    exportsList: ['bin/ng-native-engine.exe (PE x86_64)'],
    installCmd: 'Installed automatically as optionalDependency of @ng-desktop/builders',
    snippet: `"optionalDependencies": { "@ng-desktop/engine-win32-x64": "0.6.0" }`,
  },
  {
    id: 'engine-darwin-arm64',
    name: '@ng-desktop/engine-darwin-arm64',
    version: '0.6.0',
    kind: 'binary',
    summary: 'Prebuilt macOS Apple Silicon AppKit Cocoa + JavaScriptCore.framework engine binary.',
    exportsList: ['bin/ng-native-engine (Mach-O arm64)'],
    installCmd: 'Installed automatically as optionalDependency of @ng-desktop/builders',
    snippet: `"optionalDependencies": { "@ng-desktop/engine-darwin-arm64": "0.6.0" }`,
  },
  {
    id: 'engine-darwin-x64',
    name: '@ng-desktop/engine-darwin-x64',
    version: '0.6.0',
    kind: 'binary',
    summary: 'Prebuilt macOS Intel x86_64 AppKit Cocoa + JavaScriptCore.framework engine binary.',
    exportsList: ['bin/ng-native-engine (Mach-O x86_64)'],
    installCmd: 'Installed automatically as optionalDependency of @ng-desktop/builders',
    snippet: `"optionalDependencies": { "@ng-desktop/engine-darwin-x64": "0.6.0" }`,
  },
];
