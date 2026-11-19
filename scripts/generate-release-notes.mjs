#!/usr/bin/env node
import { execSync } from 'node:child_process';
import { writeFileSync } from 'node:fs';

/**
 * Generates structured, Angular-monorepo-style release notes for:
 *   1. `--kind=engine` (`v*` tags): Core Rust Engine (`ng-desktop-engine`) + Individual Angular 22+ Packages (`@ng-desktop/*`) + Demo impact summary.
 *   2. `--kind=demo` (`pokedex-demo-v*` tags): Dedicated Pokédex Nacional Desktop Showcase release with `.deb`, `.exe`, and `.dmg` native installers.
 */

const args = process.argv.slice(2);
const kindArg = args.find((a) => a.startsWith('--kind='))?.split('=')[1] ?? 'engine';
const versionArg = (args.find((a) => a.startsWith('--version='))?.split('=')[1] ?? '0.2.0').replace(
  /^(pokedex-demo-)?v/,
  '',
);
const outArg = args.find((a) => a.startsWith('--out='))?.split('=')[1];

function getRecentCommits(limit = 35) {
  try {
    const raw = execSync(`git log -n ${limit} --pretty=format:"%h|%s"`, {
      encoding: 'utf8',
    }).trim();
    if (!raw) return [];
    return raw.split('\n').map((line) => {
      const [hash, ...rest] = line.split('|');
      return { hash, subject: rest.join('|') };
    });
  } catch {
    return [];
  }
}

function categorizeCommits(commits) {
  const groups = {
    engine: [],
    platforms: [],
    desktop: [],
    http: [],
    builders: [],
    demo: [],
    ci: [],
  };

  for (const c of commits) {
    const s = c.subject.toLowerCase();
    const formatted = `- (\`${c.hash}\`) ${c.subject}`;
    if (s.includes('pokedex') || s.includes('counter-app') || s.includes('demo')) {
      groups.demo.push(formatted);
    } else if (
      s.includes('platform') ||
      s.includes('gtk') ||
      s.includes('winui') ||
      s.includes('appkit')
    ) {
      groups.platforms.push(formatted);
    } else if (s.includes('http') || s.includes('libsoup')) {
      groups.http.push(formatted);
    } else if (
      s.includes('desktop') ||
      s.includes('sqlite') ||
      s.includes('inotify') ||
      s.includes('compute')
    ) {
      groups.desktop.push(formatted);
    } else if (s.includes('builder') || s.includes('compiler') || s.includes('ast')) {
      groups.builders.push(formatted);
    } else if (
      s.includes('ci') ||
      s.includes('release') ||
      s.includes('docker') ||
      s.includes('deps')
    ) {
      groups.ci.push(formatted);
    } else {
      groups.engine.push(formatted);
    }
  }
  return groups;
}

function buildEngineReleaseNotes(version, commits) {
  const groups = categorizeCommits(commits);
  return `# 🚀 NgDesktop Engine & Angular 22+ Packages — \`v${version}\`

> **Release Scope:** Core Rust Native Rendering Engine (\`ng-desktop-engine\`) + Modular Angular 22+ Packages (\`@ng-desktop/*\`).
> Looking for the ready-to-run **Pokédex Nacional** desktop installers (\`.deb\`, \`.exe\`, \`.dmg\`)? See the companion showcase release **[Pokédex Demo \`pokedex-demo-v${version}\`](https://github.com/Gaspar1992/ng-desktop-engine/releases/tag/pokedex-demo-v${version})**.

---

## 🏛️ Architectural Overview: How the Engine & Angular Packages Work Together

\`ng-desktop-engine\` replaces the browser DOM and WebView (\`@angular/platform-browser\`) with a memory-safe **Rust scene graph** and a modular suite of **Angular 22+ packages** (\`peerDependencies: ">=22.0.0"\`):

\`\`\`
┌─────────────────────────────────────────────────────────────────────────────────┐
│                  Angular 22+ Application (Signals, DI, Pipes)                   │
├───────────────────┬────────────────────┬──────────────────┬─────────────────────┤
│ @ng-desktop/forms │ @ng-desktop/router │ @ng-desktop/http │ @ng-desktop/desktop │
├───────────────────┴────────────────────┴──────────────────┴─────────────────────┤
│              @ng-desktop/platform (NativeRenderer2 & Host Bridge)               │
├─────────────────────────────────────────────────────────────────────────────────┤
│         @ng-desktop/builders (oxc-parser TS AST + Angular Template AST)         │
└─────────────────────────────────────┬───────────────────────────────────────────┘
                                      │ Frame-Batched .ngb / FFI Calls
┌─────────────────────────────────────▼───────────────────────────────────────────┐
│               ng-desktop-engine (Rust 1.100 — Generational Arena)               │
├──────────────────────────┬──────────────────────────┬───────────────────────────┤
│    Linux GTK4/Adwaita    │     Windows WinUI 3      │    macOS AppKit/Cocoa     │
│  libgtk-4 + libsoup-3.0  │    Microsoft.UI.Xaml     │     NSWindow / NSView     │
│   libsqlite3 + inotify   │   winsqlite3 + WinHTTP   │  libsqlite3 + URLSession  │
└──────────────────────────┴──────────────────────────┴───────────────────────────┘
\`\`\`

---

## 📦 Angular 22+ Packages Breakdown (\`@ng-desktop/*\`)

Following the official Angular monorepo publishing model, each package is versioned and distributed as an independent tarball (\`peerDependencies: { "@angular/core": ">=22.0.0" }\`):

| Package | Tarball Asset | Responsibilities & Key APIs |
| :--- | :--- | :--- |
| **\`@ng-desktop/platform\`** | \`ng-desktop-platform-${version}.tgz\` | Custom \`NativeRenderer2\`, \`NativeElementNode\`, \`provideNativeImageLoader\` (\`IMAGE_LOADER\` disk cache), \`NativeTitleService\` (\`Title\`), \`NativeViewportScroller\` (\`ViewportScroller\`), \`provideSystemLocale\` (\`LOCALE_ID\` + \`DATE_PIPE_DEFAULT_TIMEZONE\`), \`NativeKeyEventsPlugin\` (\`EVENT_MANAGER_PLUGINS\`), and \`NativeSystemErrorHandler\` (\`ErrorHandler\`). |
| **\`@ng-desktop/forms\`** | \`ng-desktop-forms-${version}.tgz\` | Native \`ControlValueAccessor\` directives (\`NativeTextValueAccessor\`, \`NativeBooleanValueAccessor\`, \`NativeNumberValueAccessor\`) bridging \`FormsModule\`, \`ReactiveFormsModule\`, and Signal Forms to \`<ng-entry>\`, \`<ng-search-entry>\`, \`<ng-password-entry>\`, \`<ng-text-view>\`, \`<ng-spin-button>\`, \`<ng-switch>\`, \`<ng-checkbox>\`, \`<ng-slider>\`, and \`<ng-select>\`. |
| **\`@ng-desktop/router\`** | \`ng-desktop-router-${version}.tgz\` | DOM-free \`NativePlatformLocation\` navigation stack, \`NativeTitleStrategy\` (\`TitleStrategy\` updating OS window titlebars automatically on route navigation), and \`NativeStackRouteReuseStrategy\` (\`RouteReuseStrategy\` keeping master-detail views alive in \`<ng-stack>\`). |
| **\`@ng-desktop/http\`** | \`ng-desktop-http-${version}.tgz\` | OS-native \`NativeHttpBackend\` (\`HttpBackend\`) driving \`libsoup-3.0\` (Linux HTTP/2), \`URLSession\` (macOS), and \`WinHTTP\` (Windows), plus \`provideHttpClient\` features (\`withFetch\`, \`withInterceptors\`, \`withXsrfConfiguration\`, \`withHttpCaching\`, \`withParallelBatching\`) and reactive \`createNativeHttpResource()\`. |
| **\`@ng-desktop/desktop\`** | \`ng-desktop-desktop-${version}.tgz\` | \`NativeDesktopService\` (dark/light theme Signal, clipboard, notifications, file dialogs), \`NativeSqliteService\` (\`libsqlite3\` FFI with reactive \`liveQuery<T>()\` Signals), \`NativeNetworkService\` (\`GNetworkMonitor\` online/metered Signals), \`NativeComputeService\` (multithreaded Rust fuzzy search & 64-bit FNV-1a hashing), \`NativeFileSystemService\` (XDG dirs + atomic JSON I/O + \`inotify\` reactive file watcher), and \`NativeDropTargetDirective\` (\`[ngNativeDropTarget]\`). |
| **\`@ng-desktop/builders\`** | \`ng-desktop-builders-${version}.tgz\` | Angular 22+ CLI & Nx builder, \`oxc-parser\` AST metadata & permission analyzer (\`permissions.manifest.json\`), \`@angular/compiler\` template compiler, \`ng-desktop\` CLI, \`ng add\` schematics, prebuilt \`@ng-desktop/engine-*\` resolution, and Unix Socket HMR IPC. |

### Prebuilt Host Engine Binary Packages (\`optionalDependencies\`)

- \`ng-desktop-engine-linux-x64-gnu-${version}.tgz\`
- \`ng-desktop-engine-win32-x64-msvc-${version}.tgz\`
- \`ng-desktop-engine-darwin-arm64-${version}.tgz\`

---

## 🦀 Core Rust Engine (\`ng-desktop-engine\`)

- **Centralized Dynamic FFI Loader (\`src/ffi_dl.rs\`):** Single audited \`DynLib\` abstraction (\`dlopen\` / \`dlsym\`) shared across \`gtk4_sys.rs\`, \`jsc_sys.rs\`, \`libsoup_sys.rs\`, \`sqlite_sys.rs\`, \`system_env.rs\`, and \`fs_watcher.rs\`.
- **ThinLTO Production Profile:** Compiled with \`rustc 1.100.0\` (\`opt-level = 3\`, \`lto = "thin"\`, \`codegen-units = 1\`, \`panic = "abort"\`, \`strip = "symbols"\`), producing a **< 1 MB** standalone native runtime binary.
- **27 Cross-Platform Native Widget Primitives:** 100% parity across \`GtkLinuxBackend\` (GTK4/Libadwaita), \`WinUiBackend\` (Windows 11 WinUI 3), \`MacOsAppKitBackend\` (macOS AppKit/Cocoa), and \`HeadlessBackend\`.
- **Engine Binary Archives:**
  - \`ng-desktop-engine-linux-x86_64.tar.gz\`
  - \`ng-desktop-engine-windows-x86_64.zip\`
  - \`ng-desktop-engine-macos-arm64.tar.gz\`

---

## 🎮 Impact on the Pokédex Showcase Demo (\`examples/counter-app\`)

Every improvement in \`ng-desktop-engine\` and \`@ng-desktop/*\` directly powers the **Pokédex Nacional** desktop showcase:
1. **HTTP/2 Batching & \`createNativeHttpResource\` (\`@ng-desktop/http\`):** Loads Pokémon details and 18-card batches concurrently over persistent \`libsoup-3.0\` TLS sockets with local sprite caching.
2. **Embedded SQLite3 Favorites (\`@ng-desktop/desktop\`):** Persists trainer favorites and notes via \`NativeSqliteService.liveQuery()\`, updating the UI immediately on \`INSERT\`/\`DELETE\`.
3. **Multithreaded Fuzzy Search (\`NativeComputeService\`):** Filters the catalog off the UI thread using Rust's \`std::thread::scope\` worker pool.
4. **XDG Backup Export/Import & Live File Watcher (\`NativeFileSystemService\`):** Atomically exports and imports favorites JSON files inside XDG data directories with \`inotify\` change detection.
5. **Standalone Native Installers:** Packaged directly as installable **\`.deb\`** (Linux), **\`.exe\`** (Windows), and **\`.dmg\`** (macOS) executables.

---

## 📜 Categorized Commit Log

### Engine & Runtime (\`src/engine\`, \`src/runtime\`, \`src/ipc\`)
${groups.engine.join('\n') || '- Core scene graph and runtime maintenance'}

### Platform Backends (\`src/platforms\`, \`@ng-desktop/platform\`)
${groups.platforms.join('\n') || '- Cross-platform GTK4, WinUI 3, and AppKit parity'}

### Desktop OS Services (\`src/desktop\`, \`@ng-desktop/desktop\`)
${groups.desktop.join('\n') || '- SQLite3, inotify, and multithreaded compute pool'}

### Native HTTP/2 (\`src/desktop/http_native.rs\`, \`@ng-desktop/http\`)
${groups.http.join('\n') || '- Native HTTP/2 transport and resource signals'}

### Compiler & CLI Builders (\`@ng-desktop/builders\`)
${groups.builders.join('\n') || '- AST template, metadata, and style compilers'}

### Pokédex Showcase Demo (\`examples/counter-app\`)
${groups.demo.join('\n') || '- Pokédex showcase integration'}

### CI/CD, Containers & Packaging (\`.github/workflows\`, \`docker/\`, \`src/packager\`)
${groups.ci.join('\n') || '- Multi-OS CI and release automation'}
`;
}

function buildDemoReleaseNotes(version) {
  return `# 🎮 Pokédex Nacional — Native Desktop Showcase Demo (\`pokedex-demo-v${version}\`)

> **Built with Angular 22.2.6 + \`ng-desktop-engine\` \`v${version}\` (100% Native OS Widgets — Zero WebView / Zero Electron).**
> For the underlying Rust engine binaries and individual \`@ng-desktop/*\` Angular library tarballs, see the **[NgDesktop Engine \`v${version}\` Release](https://github.com/Gaspar1992/ng-desktop-engine/releases/tag/v${version})**.

---

## 📥 Direct Native Installers & Executables

| Operating System | Downloadable Executable / Installer | How to Install & Run |
| :--- | :--- | :--- |
| **🐧 Linux (Ubuntu / Debian x86_64)** | **\`pokedex-demo_${version}_amd64.deb\`** | \`sudo dpkg -i pokedex-demo_${version}_amd64.deb && ng-linux-counter\` (or launch **Pokédex Nacional** from your GNOME / KDE / Cosmic application menu) |
| **🪟 Windows 11 / 10 (x64)** | **\`pokedex-demo-${version}-windows-x64.exe\`** | Self-contained single-file \`.exe\` (with embedded \`.ngb\` bundle and icon). Download and run \`pokedex-demo-${version}-windows-x64.exe\` directly. |
| **🍎 macOS (Apple Silicon arm64)** | **\`pokedex-demo-${version}-macos-arm64.dmg\`** | Double-click \`pokedex-demo-${version}-macos-arm64.dmg\`, drag **\`ng-linux-counter.app\`** to Applications (or run \`Contents/MacOS/ng-linux-counter\`). |

> **Security & Least Privilege:** All installers bundle their statically analyzed \`permissions.manifest.json\` and \`ng-desktop.runtime.json\`, omitting unrestricted Windows \`runFullTrust\` / host filesystem permissions unless explicitly requested.

---

## ✨ How \`ng-desktop-engine\` \`v${version}\` Improvements Power This Demo

1. **Master-Detail Split Layout & Adaptive Chrome (\`Phase 8–10\`):**
   - Uses native \`<ng-header-bar>\` (\`AdwHeaderBar\` / \`TitleBar\` / \`NSToolbar\`), \`<ng-clamp>\` (\`AdwClamp\`), \`<ng-paned>\` resizable master-detail split view, \`<ng-flow>\` responsive Pokémon grid, \`<ng-expander>\` collapsible stats sections, and \`<ng-level-bar>\` base-stat meters (\`HP\`, \`Attack\`, \`Defense\`, \`Sp. Atk\`, \`Sp. Def\`, \`Speed\`).
2. **Native HTTP/2 PokéAPI Client & Disk Sprite Cache (\`@ng-desktop/http\`):**
   - Fetches live Pokémon data and official artwork over persistent \`libsoup-3.0\` HTTP/2 connections and caches PNG sprites in \`/tmp/ng_native_pokedex/\` for instant rendering in \`<ng-image>\` (\`GtkPicture\` / \`Image\` / \`NSImageView\`).
3. **Reactive SQLite3 Trainer Favorites (\`@ng-desktop/desktop\`):**
   - Uses \`NativeSqliteService.liveQuery()\` bound to \`libsqlite3.so.0\` (WAL mode) to store favorite Pokémon and trainer notes with zero-latency Signal updates.
4. **Multithreaded Rust Fuzzy Search (\`NativeComputeService\`):**
   - Searches Pokémon by name or National Dex number across worker threads via \`NativeComputePool::fuzzy_filter\`.
5. **XDG Favorites Export/Import & Drag-and-Drop (\`NativeFileSystemService\` & \`[ngNativeDropTarget]\`):**
   - Exports and imports favorite teams as JSON inside the OS XDG data directory with atomic file writes and kernel \`inotify\` watching.
`;
}

const commits = getRecentCommits(35);
const markdown =
  kindArg === 'demo'
    ? buildDemoReleaseNotes(versionArg)
    : buildEngineReleaseNotes(versionArg, commits);

if (outArg) {
  writeFileSync(outArg, markdown, 'utf8');
  process.stdout.write(`[Release Notes] Generated ${kindArg} release notes -> ${outArg}\n`);
} else {
  process.stdout.write(markdown);
}
