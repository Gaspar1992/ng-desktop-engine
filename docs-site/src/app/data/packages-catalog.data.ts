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
    version: '0.7.1',
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
    version: '0.7.1',
    kind: 'library',
    summary:
      'Angular CLI ng add schematic, Ahead-of-Time .ngb binary compiler (AST + scoped CSS), HMR dev server, permission analyzer, and OS packager.',
    exportsList: [
      'compileBundle',
      'parseStylesCss',
      'analyzeProjectPermissions',
      'analyzeSourcePermissions',
      'resolveWorkspaceProject',
    ],
    installCmd: 'ng add @ng-desktop/builders',
    snippet: `ng add @ng-desktop/builders
pnpm exec ng-desktop build
pnpm exec ng-desktop package --installer=deb`,
  },
  {
    id: 'forms',
    name: '@ng-desktop/forms',
    version: '0.7.1',
    kind: 'library',
    summary:
      'First-class Angular Signal Forms ([formField]) controls plus reactive/template-driven ControlValueAccessor directives for all native inputs.',
    exportsList: [
      'NATIVE_FORM_DIRECTIVES',
      'NATIVE_SIGNAL_FORM_DIRECTIVES',
      'NativeTextFieldControl',
      'NativeBooleanFieldControl',
      'NativeNumberFieldControl',
      'NativeTextValueAccessor',
    ],
    installCmd: 'pnpm add @ng-desktop/forms',
    snippet: `import { form, NATIVE_FORM_DIRECTIVES } from '@ng-desktop/forms';`,
  },
  {
    id: 'router',
    name: '@ng-desktop/router',
    version: '0.7.1',
    kind: 'library',
    summary:
      'Memory-backed hash/location navigation (provideNativeRouter) paired with NativeStackRouteReuseStrategy preserving scroll and view state.',
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
    version: '0.7.1',
    kind: 'library',
    summary:
      'Zero-CORS OS-native HTTP client backend powered by libsoup-3.0, WinHTTP, and NSURLSession (provideNativeHttpClient) plus Signal-first createNativeHttpResource.',
    exportsList: ['provideNativeHttpClient', 'NativeHttpBackend', 'createNativeHttpResource'],
    installCmd: 'pnpm add @ng-desktop/http',
    snippet: `import { createNativeHttpResource } from '@ng-desktop/http';

readonly pokemon = createNativeHttpResource(
  () => \`/api/pokemon/\${this.id()}\`,
);`,
  },
  {
    id: 'desktop',
    name: '@ng-desktop/desktop',
    version: '0.7.1',
    kind: 'library',
    summary:
      'Capability-gated desktop APIs: sandboxed filesystem I/O, local SQLite, OS dialogs, system tray menus, clipboard, shortcuts, and drag-and-drop.',
    exportsList: [
      'NativeDesktopService',
      'NativeFileSystemService',
      'NativeSqliteService',
      'NativeComputeService',
      'NativeDropTargetDirective',
    ],
    installCmd: 'pnpm add @ng-desktop/desktop',
    snippet: `const desktop = inject(NativeDesktopService);
const filePath = desktop.openFileDialog({ title: 'Select Snapshot' });`,
  },
  {
    id: 'server',
    name: '@ng-desktop/server',
    version: '0.7.1',
    kind: 'library',
    summary:
      'Local-first Nitro/h3-inspired in-process server routes backed directly by Rust SQLite, eliminating separate Node daemons in desktop apps.',
    exportsList: [
      'defineNativeEventHandler',
      'NativeServerEvent',
      'createNativeServerRouter',
      'registerNativeServerApp',
    ],
    installCmd: 'pnpm add @ng-desktop/server',
    snippet: `import { defineNativeEventHandler } from '@ng-desktop/server';

export default defineNativeEventHandler((event) => {
  return event.context.db.query('SELECT * FROM pokemon');
});`,
  },
  {
    id: 'engine-linux-x64-gnu',
    name: '@ng-desktop/engine-linux-x64-gnu',
    version: '0.7.1',
    kind: 'binary',
    summary: 'Prebuilt Linux x86_64 GTK4 + Libadwaita + QuickJS-NG / JavaScriptCore engine binary.',
    exportsList: ['bin/ng-native-engine (ELF x86_64)'],
    installCmd: 'Installed automatically as optionalDependency of @ng-desktop/builders',
    snippet: `"optionalDependencies": { "@ng-desktop/engine-linux-x64-gnu": "0.7.1" }`,
  },
  {
    id: 'engine-linux-arm64-gnu',
    name: '@ng-desktop/engine-linux-arm64-gnu',
    version: '0.7.1',
    kind: 'binary',
    summary: 'Prebuilt Linux aarch64 GTK4 + Libadwaita engine binary for ARM workstations & SBCs.',
    exportsList: ['bin/ng-native-engine (ELF aarch64)'],
    installCmd: 'Installed automatically as optionalDependency of @ng-desktop/builders',
    snippet: `"optionalDependencies": { "@ng-desktop/engine-linux-arm64-gnu": "0.7.1" }`,
  },
  {
    id: 'engine-win32-x64-msvc',
    name: '@ng-desktop/engine-win32-x64-msvc',
    version: '0.7.1',
    kind: 'binary',
    summary:
      'Prebuilt Windows 11 x86_64 Win32/WinUI + QuickJS-NG engine executable with MSIX packaging.',
    exportsList: ['bin/ng-native-engine.exe (PE x86_64)'],
    installCmd: 'Installed automatically as optionalDependency of @ng-desktop/builders',
    snippet: `"optionalDependencies": { "@ng-desktop/engine-win32-x64-msvc": "0.7.1" }`,
  },
  {
    id: 'engine-darwin-arm64',
    name: '@ng-desktop/engine-darwin-arm64',
    version: '0.7.1',
    kind: 'binary',
    summary: 'Prebuilt macOS Apple Silicon AppKit Cocoa + JavaScriptCore.framework engine binary.',
    exportsList: ['bin/ng-native-engine (Mach-O arm64)'],
    installCmd: 'Installed automatically as optionalDependency of @ng-desktop/builders',
    snippet: `"optionalDependencies": { "@ng-desktop/engine-darwin-arm64": "0.7.1" }`,
  },
  {
    id: 'engine-darwin-x64',
    name: '@ng-desktop/engine-darwin-x64',
    version: '0.7.1',
    kind: 'binary',
    summary: 'Prebuilt macOS Intel x86_64 AppKit Cocoa + JavaScriptCore.framework engine binary.',
    exportsList: ['bin/ng-native-engine (Mach-O x86_64)'],
    installCmd: 'Installed automatically as optionalDependency of @ng-desktop/builders',
    snippet: `"optionalDependencies": { "@ng-desktop/engine-darwin-x64": "0.7.1" }`,
  },
];
