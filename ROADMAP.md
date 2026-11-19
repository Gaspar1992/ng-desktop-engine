# 🗺️ NgDesktop Engine (`ng-desktop-engine`) — Architectural Roadmap to 100% Angular Compatibility

This document defines the deep technical analysis and phased engineering roadmap required to run **100% of the Angular 22+ ecosystem** (`@angular/core`, `@angular/common`, `@angular/forms`, `@angular/router`, `@angular/common/http`, `@angular/animations`, and `@angular/cdk`) natively on **Linux (GTK4 / Libadwaita)**, and progressively on **Windows (WinUI 3)** and **macOS (AppKit)**.

---

## 🧠 Deep Technical Gap Analysis: What Angular Needs Without a Browser

When `@angular/platform-browser` is removed, Angular's Ivy runtime still requires **seven foundational pillars** to operate at 100% parity:

### 1. Embedded JavaScript Engine + Unified Event Loop (`rquickjs` / V8 + `glib::MainContext`)

- **Why:** To execute real compiled Angular bundles (`main.js` output by `@angular/build:application`), the Rust binary must embed a spec-compliant ES2024+ JS engine (QuickJS-NG via `rquickjs` or V8 via `deno_core`).
- **Event Loop Unification:** Angular Zoneless change detection schedules UI refreshes via `queueMicrotask`, `Promise.then`, and `setTimeout` / `requestAnimationFrame`. The JS engine's job queue must be hooked directly into GTK4's `glib::MainContext` (`g_idle_add` / `g_timeout_add`), ensuring every microtask checkpoint flushes `RenderEngine::flush_frame()` on the main thread without race conditions.

### 2. Custom Angular Platform (`@ng-desktop/platform`) & `RendererFactory2`

- **Why:** Angular's component tree interacts with the host environment exclusively through `PlatformRef`, `RendererFactory2`, `Renderer2`, `Sanitizer`, `ErrorHandler`, and `ApplicationRef`.
- **What Must Be Implemented in TypeScript (`packages/platform-native`):**
  - `bootstrapNativeApplication(AppComponent, appConfig)`: Bootstraps Angular in **Zoneless mode** (`provideZonelessChangeDetection()`).
  - `NativeRendererFactory2` & `NativeRenderer2`: Implements all 19 methods of `Renderer2` (`createElement`, `createComment`, `createText`, `appendChild`, `insertBefore`, `removeChild`, `selectRootElement`, `parentNode`, `nextSibling`, `setAttribute`, `removeAttribute`, `addClass`, `removeClass`, `setStyle`, `removeStyle`, `setProperty`, `setValue`, `listen`) by calling the Rust FFI bridge (`globalThis.__NG_NATIVE__`) using generational `NodeId` handles.
  - **Anchor & Comment Nodes (`createComment` / `insertBefore`):** Essential for Angular's built-in control flow (`@if`, `@for`, `@switch`, `@defer`) and `<ng-container>` / `ViewContainerRef`, which rely on invisible anchor nodes in the scene graph to insert and remove dynamic views at exact child indices.

### 3. Complete Template Control Flow, Lifecycle, & Content Projection

- **Structural Directives & Control Flow (`@if`, `@for`, `@switch`, `@defer`):** Requires `insertBefore(parent, newChild, refChild)`, `removeChild(parent, oldChild)`, and virtual comment/anchor nodes in `Arena`.
- **Component Hierarchy & Content Projection (`<ng-content>`):** Nested custom components (`<app-header>`, `<app-card>`) must act as logical pass-through containers or `GtkBox` wrappers in the Rust scene graph while preserving multi-slot projection.
- **Lifecycle & Queries:** `ngOnInit`, `ngOnDestroy`, `afterNextRender`, `afterRenderEffect`, `viewChild()`, `viewChildren()`, `contentChild()`, and `ElementRef<NativeElementProxy>`.

### 4. Forms Ecosystem (`@angular/forms` + Signal Forms)

- **Why:** Angular `FormsModule` and `ReactiveFormsModule` (`FormControl`, `FormGroup`, `Validators`) do not depend on HTML `<input>` directly — they depend on **`ControlValueAccessor` (`CVA`)** directives.
- **What Must Be Implemented:**
  - Native input widgets in Rust/GTK4: `<ng-entry>` (`GtkEntry` / `GtkPasswordEntry`), `<ng-switch>` (`GtkSwitch`), `<ng-check>` (`GtkCheckButton`), `<ng-Scale>` (`GtkScale`), `<ng-dropdown>` (`GtkDropDown`), `<ng-text-view>` (`GtkTextView`).
  - Default `ControlValueAccessor` directives in `@ng-desktop/forms` that listen to native GTK4 signals (`notify::text`, `notify::active`, `notify::selected`) and call `onChange` / `onTouched`.

### 5. Navigation & Multi-Window Routing (`@angular/router`)

- **Why:** `@angular/router` defaults to `BrowserPlatformLocation` (`window.location` / `history.pushState`).
- **What Must Be Implemented:**
  - `MemoryPlatformLocation` / `NativePlatformLocation`: An in-memory URL state machine backed by Rust so `RouterLink`, `<router-outlet>`, route guards (`canActivate`, `canDeactivate`), lazy-loaded routes (`loadComponent`, `loadChildren`), and `ActivatedRoute` signals work 100% unchanged.
  - Native Linux navigation integration with `AdwNavigationView` / `AdwNavigationSplitView` / `GtkStack` transitions.

### 6. Network, Storage, & System I/O (`@angular/common/http` + OS Bridges)

- **Why:** `HttpClient` delegates network calls to the abstract `HttpBackend` token (which in browsers uses `HttpXhrBackend` or `FetchBackend`).
- **What Must Be Implemented:**
  - `NativeHttpBackend` (or global `fetch` polyfill backed by Rust/libgio/curl): Enables `HttpClient`, `httpResource()`, `rxResource()`, and all functional `HttpInterceptorFn` pipelines to execute natively off the UI thread and resolve back onto the main loop.
  - **Linux Desktop Integration (Freedesktop / XDG Portals / DBus):** Native file chooser dialogs (`GtkFileDialog`), system notifications (`GNotification`), clipboard (`GdkClipboard`), system tray (StatusNotifierItem), dark/light theme reactivity (`AdwStyleManager`), and secure keyring storage (`libsecret`).

### 7. Layout, Styling, & Animation System

- **Layout:** Support full GTK4 layout attributes (`orientation`, `spacing`, `hexpand`, `vexpand`, `halign`, `valign`, `margin-*`) plus `GtkGrid`, `GtkPaned`, `GtkScrolledWindow`, `AdwHeaderBar`, `AdwToolbarView`, and `AdwClamp`.
- **Styling:** Full component-scoped CSS encapsulation (`ViewEncapsulation.Emulated` via `.nghost-*` / `.ngcontent-*` class rewriting into `GtkCssProvider`) and CSS custom properties (`--var`).
- **Animations:** Bridge `@angular/animations` and CSS transitions into GTK4's frame clock (`GdkFrameClock`) and native `AdwTimedAnimation` / `AdwSpringAnimation`.

---

## 📋 Phased Implementation Roadmap

### Phase 1: Core Engine Foundations & Native GTK4 MVP (Completed ✅)

- [x] Generational `Arena` with `NodeId { index, generation }` and runtime `BorrowState` (`Shared` / `Exclusive`) checker.
- [x] Frame-batched `CommandQueue` and OS-agnostic `RenderCommand` pipeline.
- [x] SOLID `PlatformBackend` trait + `AnyPlatformBackend` factory (`LinuxGtkBackend`, `WinUiBackend`, `MacOsAppKitBackend`, `HeadlessBackend`).
- [x] Isolated C FFI dynamic loader (`Gtk4Sys`) for `libgtk-4.so.1`, `libgobject-2.0.so.0`, `libgio-2.0.so.0`, and `libglib-2.0.so.0`.
- [x] Official Angular 21 workspace (`@angular/core`, `@angular/cli`, `oxfmt`, `oxlint`, `angular-eslint`, `vitest`).
- [x] Multi-resolution `.ico` parser (`WindowIconAsset`) + Freedesktop `hicolor` (`16x16`..`256x256`) & Wayland/KDE Plasma taskbar integration.
- [x] Declarative template compiler (`build-bundle.mjs`) supporting initial Signals (`signal()`, `.update()`, `.set()`) and `(click)` events.

### Phase 2: Scene Graph Completeness & Layout Primitives (Completed ✅)

- [x] **Scene Graph Anchor & Sibling Operations:** Add `Comment` / `Anchor` nodes, `insert_before(parent, new_child, ref_child)`, and `remove_child(parent, child)` to `Arena`, `WidgetKind`, `RenderCommand`, and `GtkLinuxBackend`.
- [x] **Attribute & Property Pipeline (`setAttribute` / `setProperty`):** Support layout attributes directly from Angular templates (`orientation="horizontal"`, `spacing="16"`, `hexpand="true"`, `vexpand="true"`, `halign="center"`, `valign="center"`, `margin="12"`).
- [x] **Expanded GTK4 / Libadwaita Container Catalog:**
  - [x] `<ng-header-bar>` (`GtkHeaderBar` / `AdwHeaderBar`)
  - [x] `<ng-scroll>` (`GtkScrolledWindow`)
  - [x] `<ng-grid>` (`GtkGrid`)
  - [x] `<ng-separator>` (`GtkSeparator`)

### Phase 3: Embedded JS Engine & Custom `@ng-desktop/platform` (Completed ✅)

- [x] **Decoupled JS Engine Architecture (`JsEngineBackend`):** Support both Apple/WebKit **JavaScriptCore (`JscEngine` via `libjavascriptcoregtk-6.0.so.1`)** with 4-tier JIT and portable **QuickJS (`QuickJsEngine`)**, selectable via `angular.json` (`"jsEngine": "jsc" | "quickjs"`).
- [x] **Bridge `glib::MainContext` + JS Microtask & Event Queue:** Wire `setTimeout`, `clearTimeout`, `queueMicrotask`, `console.*`, and live event closures (`__dispatchEvent`) in Rust so `provideZonelessChangeDetection()` runs natively.
- [x] **Create `@ng-desktop/platform` TypeScript Package (`packages/platform-native`):**
  - [x] Implement `NativeRendererFactory2` and `NativeRenderer2` (all 19 `Renderer2` methods) over `globalThis.__NG_NATIVE__`.
  - [x] Support Angular `@if`, `@for`, `@switch`, and `@defer` blocks using `insert_before` and comment/anchor nodes.
  - [x] Provide `provideNativePlatform()` and `bootstrapNativeApplication()`.

### Phase 4: Interactive Controls & `@angular/forms` Compatibility (Completed ✅)

- [x] **Native Input Widgets in `GtkLinuxBackend`:**
  - [x] `<ng-entry>` (`GtkEntry` with `changed` / `notify::text` signals)
  - [x] `<ng-switch>` (`GtkSwitch` with `state-set` / `notify::active` signals)
  - [x] `<ng-checkbox>` (`GtkCheckButton` with `toggled` signal)
  - [x] `<ng-slider>` (`GtkScale` with `value-changed` signal)
  - [x] `<ng-select>` (`GtkDropDown` with `notify::selected` signal)
- [x] **`@ng-desktop/forms` ControlValueAccessors (`packages/forms-native`):**
  - [x] Implement `NativeTextValueAccessor`, `NativeBooleanValueAccessor`, and `NativeNumberValueAccessor` for native widgets.
  - [x] Full compatibility with `FormsModule` (`[(ngModel)]`), `ReactiveFormsModule` (`formControl`, `formGroup`, `Validators`), and Angular Signal Forms.

### Phase 5: `@angular/router`, `@angular/common/http`, & Desktop OS APIs (Completed ✅)

- [x] **`@ng-desktop/router` (`NativePlatformLocation` — `packages/router-native`):**
  - [x] Support `<router-outlet>`, `routerLink`, route guards, resolvers, and lazy loading without `window.location`.
- [x] **`@ng-desktop/http` (`NativeHttpBackend` — `packages/http-native`):**
  - [x] Implement HTTP client bridge in Rust (`DesktopServices` & `__NG_NATIVE__.httpRequest`) feeding `HttpClient`, `HttpInterceptorFn`, and `httpResource()`.
- [x] **Linux Desktop System Services (`@ng-desktop/desktop` — `packages/desktop-native`):**
  - [x] Native File Dialogs (`openFileDialog` / `saveFileDialog`).
  - [x] Desktop Notifications (`notify`) & System Clipboard (`getClipboardText` / `setClipboardText`).
  - [x] System Dark/Light Theme Signal (`theme` WritableSignal) synchronized with `DesktopServices`.

### Phase 6: Packaging & Cross-Platform Native Backends (Windows & macOS) (Completed ✅)

- [x] **Angular 22 CLI Builder (`@ng-desktop/builders` — `packages/builders-native`):** `createNativeTargetConfig` and `executeNativeBuilder` integration with `ng build` / `pnpm run package:linux`.
- [x] **Linux Distribution Bundler (`src/packager/mod.rs`):** Generate self-contained `.deb` (`DEBIAN/control` + Freedesktop `.desktop`), `AppImage` (`AppRun`), and `Flatpak` (`.yml`) distribution artifacts with `hicolor` icon sets (`cargo run -- --package`).
- [x] **Windows Native Backend (`WinUiBackend`):** Stateful `NativeWinUiControl` scene tree (`Microsoft.UI.Xaml.*`) and command processor.
- [x] **macOS Native Backend (`MacOsAppKitBackend`):** Stateful `NativeAppKitView` scene tree (`NSWindow` / `NSStackView` / `NSTextField` / `NSButton`) and command processor.

### Phase 7: Post-Phase 6 Hardening, Modern AST Toolchain, & Real-Time IPC (Completed ✅)

- [x] **AST-Driven Compilation Pipeline (Zero Regex Parsing):**
  - [x] **TypeScript 6.0 AST:** Implemented `ast-metadata-extractor.mjs` using Rust-based `oxc-parser` (`parseSync`) to extract component metadata, signals, and updater methods directly from the syntax tree.
  - [x] **Angular Template AST:** Implemented `template-compiler.mjs` using `@angular/compiler`'s official `parseTemplate` AST, resolving control flow (`@if`), dynamic inputs, reactive text bindings, and events.
  - [x] **Native CSS AST:** Implemented `style-compiler.mjs` using `postcss` to parse standard CSS (`padding`, `gap`, `border-radius`, `font-size`, `font-weight`) into typed `StyleRule` definitions.
- [x] **Cross-Platform Retaining Scene State (`src/platforms/scene_state.rs`):**
  - [x] Extracted DRY `NativeSceneState` and `NativeSceneNode` shared between `WinUiBackend` and `MacOsAppKitBackend`, reducing adapter size by ~50% and supporting styles and window icons.
- [x] **Real-Time Unix Domain Socket HMR IPC (`src/ipc/mod.rs` & `hmr-client.mjs`):**
  - [x] Non-blocking IPC server in Rust handling `HmrMessage::Reload`, `HmrMessage::Eval`, and `HmrMessage::Ping` over local Unix domain sockets (`dist/<project>/hmr.sock`).
  - [x] Node.js IPC client pushing live `.ngb` bundles and evaluating JS closures directly in the running GTK4 window without file-polling latency or window restarts.
- [x] **Single Responsibility Principle (SRP) Modularization:**
  - [x] Split `src/engine/widget.rs` by extracting `src/engine/widget_attrs.rs`.
  - [x] Split `src/runtime/mod.rs` into `js_bridge.rs`, `reactivity.rs`, and `scene_exec.rs`.
- [x] **Angular 22 & TypeScript 6.0 Workspace Upgrade:**
  - [x] Upgraded root workspace and all packages to `@angular/*@22.2.5`, `@angular/build@22.2.6`, and `typescript@6.0.3`.
- [x] **Media, Meter, Feedback, & Responsive Flow Primitives:**
  - [x] Added `<ng-image>` (`GtkPicture`), `<ng-level-bar>` (`GtkLevelBar`), `<ng-flow>` (`GtkFlowBox`), `<ng-spinner>` (`GtkSpinner`), and `<ng-progress-bar>` (`GtkProgressBar` with indeterminate pulsing).
- [x] **Modular Angular Components & Content Projection:**
  - [x] Support nested custom components (`<app-pokemon-card>`, `<app-type-badge>`) with `<ng-content>` projection and collision-free `idState` generation across expansions.
- [x] **Live Pokédex Desktop Showcase (`PokedexController`):**
  - [x] Non-blocking worker-thread PokéAPI client (`src/desktop/pokedex.rs` & `src/desktop/pokedex_ui.rs`) with 500ms search debounce, `<ng-spinner>` loading feedback, and `JsRuntime::set_signal`.
- [x] **Comprehensive Automated Test Coverage:**
  - [x] 66 automated tests passing (25 Rust unit tests + 14 Rust E2E integration tests + 27 Angular Vitest tests).

### Phase 8: Advanced Layout Containers & Extended Primitives (Completed ✅)

- [x] **Priority Layout & Container Primitives:**
  - [x] `<ng-paned>` (`GtkPaned` / `Microsoft.UI.Xaml.Controls.SplitView` / `NSSplitView`): Resizable two-pane split container supporting `orientation` (`horizontal` | `vertical`), divider `position` (px), and start/end child slots.
  - [x] `<ng-stack>` (`GtkStack` / `Microsoft.UI.Xaml.Controls.NavigationView` / `NSTabView`): Multi-page view container supporting indexed named child views (`page_0`...), `visible-child` / `active-page` switching, and `transition` / `transition-type`.
  - [x] `<ng-expander>` (`GtkExpander` / `Microsoft.UI.Xaml.Controls.Expander` / `NSDisclosureButton`): Collapsible disclosure/accordion container supporting `label` / `title`, `expanded` / `open` state attribute, and `notify::expanded` signal emission.
- [x] **Extended Form & Adaptive Primitives:**
  - [x] `<ng-text-view>` (`GtkTextView` multi-line text editor with `editable`, `monospace`, and `NativeTextValueAccessor` support)
  - [x] `<ng-password-entry>` (`GtkPasswordEntry` with `show-peek-icon` / `peek` and `NativeTextValueAccessor` support)
  - [x] `<ng-spin-button>` (`GtkSpinButton` numeric stepper with `min`, `max`, `step`, `value`, and `NativeNumberValueAccessor` support)
  - [x] `<ng-search-entry>` (`GtkSearchEntry` with native search icon, `placeholder`, and `NativeTextValueAccessor` support)
  - [x] `<ng-clamp>` (`AdwClamp` responsive maximum-width container with `max-size` / `maximum-size`)
- [x] **Pokédex Master-Detail Split Upgrade (`examples/counter-app` & `PokedexController`):**
  - [x] Upgraded the showcase to use `<ng-clamp>`, `<ng-search-entry>`, `<ng-spin-button>`, `<ng-paned>` master-detail layout, collapsible `<ng-expander>` sections, and `<ng-text-view>` trainer notes.
- [x] **Automated Test Coverage:** 70 automated tests passing (27 Rust unit tests + 15 Rust E2E integration tests + 28 Angular Vitest tests).

---

### Phase 9: OS-Native HTTP/2, `@ng-desktop/builders` Toolchain, SQLite FFI, & Full Angular Platform Integration (Completed ✅)

- [x] **OS-Native HTTP/2 & `provideHttpClient` Parity (`@ng-desktop/http` + `src/desktop/http_native.rs`):**
  - [x] `libsoup-3.0.so.0` FFI session (`libsoup_sys.rs`) with persistent TLS/HTTP2 sockets, `http_codec.rs` JSON bridge separation (SRP), parallel HTTP/2 batch transfers, and `createNativeHttpResource()`.
  - [x] JSC C callbacks (`__NG_NATIVE_HOST_HTTP__`, `__NG_NATIVE_HOST_HTTP_CONFIG__`) wired to Angular `provideHttpClient(...)` (`withFetch`, `withInterceptors`, `withXsrfConfiguration`, `withHttpCaching`, `withParallelBatching`).
- [x] **Toolchain Extraction & Minimal `angular.json` Schema (`@ng-desktop/builders`):**
  - [x] Extracted all AST compilers, HMR client, and dev-server out of `examples/counter-app` into `packages/builders-native/`, leaving `examples/counter-app` as a pure Angular 22 project.
  - [x] Added `angular-native-schema.json`, `native-builder-schema.json`, and minimal `{ os, arch, cpu }` config derivation in `builder.ts` and `src/config/mod.rs`.
- [x] **Runtime Reactive Control Flow & Angular 22 Reactivity:**
  - [x] Implemented `.ngb` opcodes `bind_visible` and `bind_attr` across `parser.rs`, `reactivity.rs`, `bootstrap_js.rs`, `bridge.ts`, and `quickjs_engine.rs` for reactive `@if`/`@else`, `@switch`, `[visible]`, `[attr]`, and parent-child input/output forwarding.
  - [x] Migrated Angular Pipes, `@let`, `@for`/`@empty`, `@defer`/`@error`, `computed()`, `linkedSignal()`, `input()`, `model()`, `output()`, and `httpResource()`.
- [x] **Nivel 1 & Nivel 2 OS/Angular Integrations:**
  - [x] **SQLite3 FFI & Live Queries (`src/desktop/sqlite_sys.rs`, `@ng-desktop/desktop`):** `NativeSqliteDatabase` (`libsqlite3.so.0`, WAL mode) + Signal-based `NativeSqliteService.liveQuery()`.
  - [x] **OS Environment & Network Monitor (`src/desktop/system_env.rs`, `@ng-desktop/desktop`):** `GNetworkMonitor` (`NativeNetworkService`), `GLib` locale/timezone (`provideSystemLocale()`), disk image cache (`provideNativeImageLoader()`), `NativeTitleService` (`Title`), `NativeTitleStrategy` (`TitleStrategy`), `NativeStackRouteReuseStrategy` (`RouteReuseStrategy`), `NativeViewportScroller` (`ViewportScroller`), `NativeKeyEventsPlugin` (`EVENT_MANAGER_PLUGINS`), and `NativeSystemErrorHandler` (`ErrorHandler`).
- [x] **CI & Coverage:** Green GitHub Actions CI with Linux native shared libraries (`libgtk-4-1`, `libadwaita-1-0`, `libjavascriptcoregtk-6.0-1`, `libsoup-3.0-0`, `libsqlite3-0`) and 93 automated tests passing (38 Rust unit + 16 Rust integration + 39 Vitest).

---

### Phase 10 (Nivel 3): Multithreaded Compute Pool, Zero-Copy File System & Kernel Watcher (`inotify`), Native Drag & Drop, and Multi-Container Testing (Completed ✅)

- [x] **Multithreaded Rust Compute Pool (`src/desktop/compute_pool.rs` + `packages/desktop-native/src/compute-service.ts`):**
  - [x] Implemented `NativeComputePool` using `std::thread::scope` parallel worker chunks for off-UI-thread fuzzy filtering (`fuzzy_filter`, `fuzzy_filter_json`) and 64-bit FNV-1a hashing (`fnv1a_hex`), exposed via `DesktopServices` and `__NG_NATIVE__` (`computeFuzzyFilter`, `computeHash`).
  - [x] Implemented Angular `NativeComputeService` with `fuzzyFilter<T>()`, `fuzzyFilterStrings()`, and `hash()`.
- [x] **XDG File System, Atomic I/O & Kernel File Watcher (`src/desktop/fs_watcher.rs` + `packages/desktop-native/src/fs-service.ts`):**
  - [x] Implemented `NativeFsManager` with XDG base dir resolution (`XdgDirKind::{Config, Data, Cache}` + traversal protection), atomic file writes (`write_text_atomic` via `.tmp.<pid>` + `fs::rename`), and Linux `inotify` (`inotify_init1(IN_NONBLOCK)` + `inotify_add_watch` via `libc.so.6` dynamic FFI) with cross-platform `OsFileWatcherBackend` tracking (`LinuxInotify`, `WindowsReadDirectoryChanges`, `MacOsFsEvents`).
  - [x] Exposed `fsResolvePath`, `fsReadText`, `fsWriteText`, `fsWatchFile`, and `fsPollChangedFiles` on `NgNativeBridge` (`bridge.ts` + `bridge-fs-compute.ts` + `bootstrap_js.rs` + `quickjs_engine.rs`) and wrapped in Angular `NativeFileSystemService` (`resolveXdgPath`, `readText`, `writeText`, `readJson`, `writeJson`, `watchFile`, `watchJson`).
- [x] **Native Drag & Drop (`GtkDropTarget` + `NativeDropTargetDirective`) & Cross-Platform Parity:**
  - [x] Extracted `src/platforms/gtk4_controls.rs` from `gtk4_widgets.rs` (SRP) and completed all 27 widget mappings in `PlatformBackend::map_native_widget`.
  - [x] Synchronized `drop-target`, `scroll-to`, and `scroll-anchor` attributes + native event queues across `GtkLinuxBackend`, `WinUiBackend`, and `MacOsAppKitBackend` (`NativeSceneState`), and added explicit OS driver enums (`OsHttpPlatform`, `OsFileWatcherBackend`, `OsSqliteBackend`, `OsSystemEnvBackend`).
  - [x] Created `NativeDropTargetDirective` (`[ngNativeDropTarget]`, `fileDrop = output<string[]>()`) in `packages/desktop-native/src/drop-target.directive.ts` and wired `PokedexService` (`exportFavoritesToDisk`, `importFavoritesFromFile`, `filterCatalog`).
- [x] **Multi-Container & Xvfb CI Testing Infrastructure + Automated Multi-OS GitHub Releases:**
  - [x] Added `docker/Dockerfile.ubuntu-gtk4` (Ubuntu 24.04), `docker/Dockerfile.fedora-gtk4` (Fedora 41), `docker/Dockerfile.windows-wine` (Windows x86_64 `.exe` cross-compile + `wine64` execution), `docker/Dockerfile.macos-cross` (macOS Apple Silicon & Intel target checks), `docker/docker-compose.kvm-os.yml` (`dockurr/windows` & `dockurr/macos` KVM desktop VMs), and `scripts/test-containers.sh` (`pnpm run test:containers`).
  - [x] Added multi-OS distribution staging (`stage_windows_distribution`, `stage_macos_distribution`, `stage_all_distributions`) in `src/packager/mod.rs` (`pnpm run package:all`).
  - [x] Configured multi-OS CI matrix (`ubuntu-24.04`, `windows-latest`, `macos-latest`) in `.github/workflows/ci.yml` and automated GitHub Release generation (`ng-linux-counter-linux-x86_64-gtk4.tar.gz`, `ng-linux-counter-windows-x86_64-winui3.zip`, `ng-linux-counter-macos-arm64-appkit.tar.gz`, `SHA256SUMS.txt`) in `.github/workflows/release.yml`.
  - [x] Expanded automated coverage to **102 passing tests** (40 Rust unit + 19 Rust integration + 43 Vitest) with zero warnings across Linux, Windows, and macOS targets.

---

### Phase 11: Zero Hardcoded Literals, Nx + Least-Privilege Permissions, APF Libraries & Ecosystem Compatibility (Completed ✅)

- [x] **Workspace-Agnostic Configuration & Zero Hardcoded Literals (`src/config/mod.rs` + `packages/builders-native/src/workspace-resolver.mjs`):**
  - [x] Removed hardcoded `"ng-linux-counter"`, `"angular.json"`, and `"0.2.0"` defaults across `src/main.rs`, `src/desktop/mod.rs`, `src/platforms/gtk4_sys.rs`, `src/platforms/linux_gtk.rs`, `src/runtime/bootstrap_js.rs`, and `packages/builders-native/src/build-bundle.mjs`.
  - [x] Added transparent config resolution supporting **Angular CLI (`angular.json`)**, **Nx (`nx.json` / `project.json`)**, runtime manifests (`ng-desktop.runtime.json`), CLI flags (`--config`, `--project`, `--version`, `--output-path`, `--target-platform`), and `NG_DESKTOP_*` env vars.
- [x] **Static Build-Time Permission Analysis & Least-Privilege Sandboxing (`src/config/permissions.rs` + `packages/builders-native/src/permission-analyzer.mjs`):**
  - [x] Added AST static analyzer powered by `oxc-parser` to infer OS capabilities (`network`, `sqlite`, `filesystem:xdg`, `filesystem:dialogs`, `filesystem:drop`, `clipboard`, `notifications`, `compute-threads`) with `mode: 'auto' | 'strict'` and `allow`/`deny` overrides in `angular.json` / `project.json`.
  - [x] Removed unconditional Windows `<rescap:Capability Name="runFullTrust" />` and unrestricted host capabilities unless explicitly opted into (`fullSystemAccess: true`), emitting minimal Windows `AppxManifest.xml`, macOS `Entitlements.plist` (`com.apple.security.app-sandbox`), Linux `AppArmor` profiles, and Flatpak manifests alongside runtime enforcement in `DesktopServices`.
- [x] **Ecosystem Compatibility Bridges (`@ng-desktop/platform` + `docs/ECOSYSTEM_COMPATIBILITY.md`):**
  - [x] Extended `NativeElementNode` with WAI-ARIA/DOM attributes, focus tracking, `classList`, `contains()`, event listeners, and `getBoundingClientRect()` for `@angular/aria` and `@angular/cdk/a11y`.
  - [x] Added `NativeStorage` (`Storage` over SQLite), `NativeAriaAnnouncer` (`LiveAnnouncer`), `NativeDevToolsHook` (`__REDUX_DEVTOOLS_EXTENSION__`), `createNativeDocument()`, and `provideNativeEcosystemCompat()` for `@ngrx/signals`, `@ngrx/store`, `@ngrx/store-devtools`, and headless CDK/Aria libraries, plus documented `@angular/material` rendering gaps.
- [x] **Angular Package Format (`APF`) Build Pipeline, `ng-desktop` CLI & `ng add` Schematic (`pnpm run build:packages`):**
  - [x] Added `tsconfig.lib.json` (`compilationMode: "partial"`) and `scripts/build-packages.mjs` compiling all 6 `@ng-desktop/*` packages into `dist/packages/*` ESM + `.d.ts` APF bundles packed in `.github/workflows/release.yml`.
  - [x] Added `ng-desktop` CLI (`packages/builders-native/bin/ng-desktop.mjs`) and `ng add @ng-desktop/builders` schematic (`packages/builders-native/schematics/ng-add`) supporting both Angular CLI and Nx.

---

### Phase 12: Automated npm / `crates.io` Publishing & Prebuilt Native Binary Distribution (Completed ✅)

- [x] **Prebuilt Cross-Platform Rust Engine Binaries & Resolver (`packages/builders-native/src/resolve-engine-binary.mjs` + `scripts/build-engine-packages.mjs`):**
  - [x] Staged platform binary npm packages (`@ng-desktop/engine-linux-x64-gnu`, `@ng-desktop/engine-linux-arm64-gnu`, `@ng-desktop/engine-win32-x64-msvc`, `@ng-desktop/engine-darwin-arm64`, `@ng-desktop/engine-darwin-x64`) pinned in `optionalDependencies` of `@ng-desktop/builders`.
  - [x] Implemented `resolveEngineBinaryPath()` (`NG_DESKTOP_ENGINE_BIN` -> installed `@ng-desktop/engine-*` package -> local Cargo target -> `cargo run --` fallback) integrated across `dev-server.mjs`, `ng-desktop run`, `ng-desktop package`, and `builders.json` (`--noEmit` APF cross-OS `process.execPath` build fix).
- [x] **External `npm pack` Verification & Automated Publishing (`scripts/test-external-pack.mjs`, `Cargo.toml`, `.github/workflows/release.yml`):**
  - [x] Enriched `Cargo.toml` metadata for `crates.io` (`description`, `license`, `repository`, `documentation`, `keywords`, `categories`, scoped `include`) and APF npm manifests (`repository`, `publishConfig`, `README.md`, `LICENSE`).
  - [x] Added `pnpm run test:pack` verifying tarball contents and out-of-monorepo import execution of packed `@ng-desktop/builders` across **121 passing tests** (51 Vitest + 50 Rust unit + 20 Rust integration).
  - [x] Added `publish-npm` (OIDC `--provenance` or safe `--dry-run`) and `publish-crates` (`cargo publish --locked` or safe `--dry-run`) jobs to `.github/workflows/release.yml`.

---

### Phase 13: Official Documentation Website on GitHub Pages (Planned 🌐)

1. **Angular 22 Interactive Docs Portal (`docs-site/` -> `dist/ng-native-docs`):**
   - [ ] Create the `ng-native-docs` Angular 22 application (`--base-href /ng-native-engine/`) featuring an interactive 27-primitive explorer, live `.ngb` template compiler playground, architecture guides, and Pokédex tutorial.
2. **Automated TypeScript & Rust API Reference (`scripts/build-api-docs.mjs`):**
   - [ ] Bundle `@ng-desktop/*` TypeScript API reference (`/api-ts/`), `cargo doc --no-deps` Rust engine documentation (`/api-rust/`), `.nojekyll`, and `404.html` SPA routing fallback into `dist/ng-native-docs/browser`.
3. **GitHub Pages Continuous Deployment (`.github/workflows/docs-pages.yml`):**
   - [ ] Automatically build and deploy `dist/ng-native-docs/browser` to `https://gaspar1992.github.io/ng-native-engine/` via `actions/upload-pages-artifact@v3` and `actions/deploy-pages@v4` on every push to `main`.
