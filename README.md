# 🚀 NgDesktop Engine (`ng-desktop-engine`)

A high-performance, memory-safe **Native Desktop Rendering Engine for Angular 22+**, written in **Rust**.

`ng-desktop-engine` replaces `@angular/platform-browser` and the browser DOM/WebView completely. Instead of bundling Chromium (Electron) or relying on an embedded HTML WebView (Tauri), Angular components drive **100% native OS widgets** across **Linux (GTK4 / Libadwaita)**, **Windows 11 (Win32 / WinUI)**, and **macOS (Cocoa / AppKit)** using ES2024+ embedded JS runtimes, a shared Rust layout solver, and native OS service drivers (see the [Platform Maturity Matrix](./ROADMAP.md#platform-maturity-matrix-v060)).

> **Using `@ng-desktop/*` in your own app?** Browse the interactive [**Official Documentation Portal**](https://gaspar1992.github.io/ng-desktop-engine/) (with live **`.ngb` Playground**, **29-Primitive Explorer**, [`/api-ts/`](https://gaspar1992.github.io/ng-desktop-engine/api-ts/) & [`/api-rust/`](https://gaspar1992.github.io/ng-desktop-engine/api-rust/)), or jump straight to the [**External Angular 22+ & Nx Tutorial (`docs/GETTING_STARTED.md`)**](./docs/GETTING_STARTED.md).

---

## 🎯 Architectural Principles

1. **Zero WebView Overhead & Zero `CUSTOM_ELEMENTS_SCHEMA`:** Angular templates compile and execute against **29 native OS primitives + `<ng-anchor>`** (`AdwApplicationWindow`, `AdwDialog`, `GtkHeaderBar`, `AdwClamp`, `GtkBox`, `GtkGrid`, `GtkFlowBox`, `GtkPaned`, `GtkStack`, `GtkExpander`, `GtkScrolledWindow`, `GtkListBox`, `GtkPopover`, `GtkSeparator`, `GtkLabel`, `GtkPicture`, `GtkLevelBar`, `GtkProgressBar`, `GtkSpinner`, `GtkButton`, `GtkEntry`, `GtkSearchEntry`, `GtkPasswordEntry`, `GtkTextView`, `GtkSpinButton`, `GtkSwitch`, `GtkCheckButton`, `GtkScale`, `GtkDropDown`). Importing `NATIVE_UI_DIRECTIVES` from `@ng-desktop/platform` provides typed bindings for both `<ng-*>` widgets and standard HTML tags (`<div>`, `<button>`, `<span>`, `<table>`, `<dialog>`, `<menu>` via `HTML_TO_NATIVE_TAG`) without `CUSTOM_ELEMENTS_SCHEMA`.
2. **Rust-Grade Memory Safety Across the JS/Native Boundary:**
   - **Generational Arena (`Arena` + `NodeId`):** JavaScript never holds raw C/OS pointers. Every native widget lives inside a Rust generational arena (`index` + `generation`), preventing use-after-free bugs when components unmount.
   - **Runtime Borrow Checker (`BorrowState`):** Enforces Rust's aliasing rules (`Shared(usize)` vs `Exclusive`) at runtime across asynchronous UI events, blocking illegal node destruction while an event callback is actively borrowing a widget.
3. **Game-Engine Inspired Render Pipeline & Scalable Events (`CommandQueue` + `EventMask`):**
   - Angular mutations during a change-detection tick batch into an OS-agnostic [`RenderCommand`](./src/engine/commands.rs) buffer and flush atomically per frame, while [`EventMask` & `coalesce_frame_events`](./src/engine/events.rs) lazily attach OS event controllers and coalesce noisy continuous streams (scroll/motion), supporting `(closeRequest)` / `(beforeunload)` hooks and scroll telemetry.
4. **AST-Driven Compilation & Embedded Server Pipeline (Zero Regex Parsing):**
   - **TypeScript 6.0 AST:** Powered by Rust-based [`oxc-parser`](./packages/builders-native/src/ast-metadata-extractor.mjs), extracting component decorators, `signal()`, `computed()`, `linkedSignal()`, `input()`, `model()`, `output()`, `httpResource()`, and reactive mutation methods.
   - **Angular Template AST:** Powered by official [`@angular/compiler`](./packages/builders-native/src/template-compiler.mjs) `parseTemplate`, compiling control flow (`@if`/`@else`, `@for`/`@empty`, `@switch`, `@let`, `@defer`/`@error`), Angular pipes, dynamic inputs, reactive `bind_visible`/`bind_attr` bindings, `<router-outlet />`, and events.
   - **CSS Flexbox/Grid Projector:** Unified [`css-style-parser.ts`](./packages/platform-native/src/css-style-parser.ts) + [`postcss` style compiler](./packages/builders-native/src/style-compiler.mjs), projecting `display: flex`, `display: grid`, `flex-direction`, `flex-wrap`, `gap`, `min-width`, `min-height`, `padding`, `border-radius`, and typography onto native GTK4 / WinUI / AppKit layouts.
   - **Embedded Local Server (`@ng-desktop/server`):** Compiles file-based API routes under `src/server/routes/` into `server.bundle.js` executed by the Rust [`server_worker`](./src/runtime/server_worker.rs).
5. **Real-Time Cross-Platform HMR IPC:**
   - Non-blocking IPC server ([`src/ipc/`](./src/ipc/mod.rs)) communicating over Unix Domain Sockets (Linux/macOS) or loopback TCP pointer files (Windows) with a Node.js client ([`hmr-client.mjs`](./packages/builders-native/src/hmr-client.mjs)) for zero-latency, atomic hot module reloads while preserving active signal state.
6. **SOLID & Hexagonal Platform Architecture (`PlatformBackend`):**
   - The core [`RenderEngine<B: PlatformBackend>`](./src/engine/mod.rs) depends exclusively on an abstract trait, never on concrete OS toolkits.
   - Concrete platform adapters live in [`src/platforms/`](./src/platforms/):
     - **Linux GTK4 (`GtkLinuxBackend`):** Full Ivy runtime via [`gtk4_sys.rs`](./src/platforms/gtk4_sys.rs) (`libgtk-4.so.1`, `libadwaita-1.so.0`, `libjavascriptcoregtk-6.0.so.1`).
     - **Windows Win32 / WinUI (`WinUiBackend`):** Live `user32.dll` / `kernel32.dll` FFI realization via [`win32_sys.rs`](./src/platforms/win32_sys.rs), shared Flexbox/Grid layout solver ([`layout_solver.rs`](./src/platforms/layout_solver.rs)), `WM_COMMAND` container event routing, and embedded `QuickJS-NG` ES2024+ execution.
     - **macOS AppKit (`MacOsAppKitBackend`):** Live Objective-C runtime FFI realization via [`appkit_sys.rs`](./src/platforms/appkit_sys.rs), shared Y-flipped Cocoa layout positioning, `NgAppKitEventTarget` / delegate callbacks, and Apple `JavaScriptCore.framework` ES2024+ execution.
     - **Headless CI (`HeadlessBackend`):** Records every frame's `RenderCommand` batch in memory for deterministic integration testing in CI without a display server.

---

## 🧩 Native Primitives Catalog

### Implemented Native Primitives (29 Primitives + `<ng-anchor>`)

| Tag (Aliases)                                            | Category             | Linux (GTK4 / Libadwaita) | Windows (WinUI 3)                            | macOS (AppKit)        | Attributes & Events                                                                                                               |
| :------------------------------------------------------- | :------------------- | :------------------------ | :------------------------------------------- | :-------------------- | :-------------------------------------------------------------------------------------------------------------------------------- |
| `<ng-window>` (`adw-window`)                             | Window               | `AdwApplicationWindow`    | `Microsoft.UI.Xaml.Window`                   | `NSWindow`            | `title`, icon, `resizable`, `decorated`, `prevent-close`, `close-window`, `(closeRequest)`, `(beforeunload)`                      |
| `<ng-dialog>` (`dialog`, `gtk-dialog`, `adw-dialog`)     | Dialog / Modal       | `AdwDialog`               | `Microsoft.UI.Xaml.Controls.ContentDialog`   | `NSPanel`             | `title`, `modal`, `transient-for`, `prevent-close`, `(closeRequest)`, `(closed)`                                                  |
| `<ng-header-bar>` (`adw-header-bar`, `gtk-header-bar`)   | Window Chrome        | `AdwHeaderBar`            | `Microsoft.UI.Xaml.Controls.TitleBar`        | `NSToolbar`           | `title`, `subtitle`, `show-title-buttons`                                                                                         |
| `<ng-clamp>` (`adw-clamp`)                               | Adaptive Container   | `AdwClamp`                | `Microsoft.UI.Xaml.Controls.Grid`            | `NSBox`               | `max-size` / `maximum-size` (px), single centered child                                                                           |
| `<ng-box>` (`div`, `gtk-box`)                            | Layout Container     | `GtkBox`                  | `Microsoft.UI.Xaml.Controls.StackPanel`      | `NSStackView`         | `orientation` (`vertical` \| `horizontal`), `spacing` / `gap`                                                                     |
| `<ng-grid>` (`gtk-grid`)                                 | Layout Container     | `GtkGrid`                 | `Microsoft.UI.Xaml.Controls.Grid`            | `NSGridView`          | `spacing` / `gap`, `row-spacing`, `column-spacing`                                                                                |
| `<ng-flow>` (`ng-flow-box`, `gtk-flow-box`)              | Responsive Layout    | `GtkFlowBox`              | `Microsoft.UI.Xaml.Controls.ItemsRepeater`   | `NSCollectionView`    | `spacing` / `gap`, `row-spacing`, `column-spacing`, `min-children` / `min-per-row`, `max-children` / `max-per-row`, `homogeneous` |
| `<ng-paned>` (`gtk-paned`)                               | Split Container      | `GtkPaned`                | `Microsoft.UI.Xaml.Controls.SplitView`       | `NSSplitView`         | `orientation` (`horizontal` \| `vertical`), `position` (px), start/end children                                                   |
| `<ng-stack>` (`gtk-stack`, `adw-navigation-view`)        | Multi-Page Container | `GtkStack`                | `Microsoft.UI.Xaml.Controls.NavigationView`  | `NSTabView`           | `visible-child` / `active-page` (`page_0`...), `transition` / `transition-type`                                                   |
| `<ng-expander>` (`details`, `gtk-expander`)              | Disclosure Container | `GtkExpander`             | `Microsoft.UI.Xaml.Controls.Expander`        | `NSDisclosureButton`  | `label` / `title`, `expanded` / `open`, `notify::expanded`                                                                        |
| `<ng-scroll>` (`gtk-scrolled-window`)                    | Scroll Container     | `GtkScrolledWindow`       | `Microsoft.UI.Xaml.Controls.ScrollViewer`    | `NSScrollView`        | `min-height`, `(scroll)`, `(scrollTop)`, `(scrollBottom)`, `(scrollEdge)`                                                         |
| `<ng-list>` (`ng-table`, `table`, `gtk-list-box`)        | List / Table         | `GtkListBox`              | `Microsoft.UI.Xaml.Controls.ListView`        | `NSTableView`         | `selection-mode` (`none`\|`single`\|`browse`\|`multiple`), `show-separators`, `(rowActivated)`                                    |
| `<ng-popover>` (`ng-menu`, `menu`, `gtk-popover`)        | Anchored Popover     | `GtkPopover`              | `Microsoft.UI.Xaml.Controls.Flyout`          | `NSPopover`           | `anchor`, `open`, `position` / `placement`, `autohide`, `has-arrow`, `(closed)`                                                   |
| `<ng-separator>` (`hr`, `gtk-separator`)                 | Layout Divider       | `GtkSeparator`            | `Microsoft.UI.Xaml.Controls.AppBarSeparator` | `NSBoxSeparator`      | `orientation` (`horizontal` \| `vertical`)                                                                                        |
| `<ng-label>` (`p`, `span`, `h1`..`h6`, `gtk-label`)      | Typography           | `GtkLabel`                | `Microsoft.UI.Xaml.Controls.TextBlock`       | `NSTextField`         | text content / `{{ signal() }}`                                                                                                   |
| `<ng-image>` (`img`, `ng-picture`, `gtk-picture`)        | Media                | `GtkPicture`              | `Microsoft.UI.Xaml.Controls.Image`           | `NSImageView`         | `src`, `width`, `height`                                                                                                          |
| `<ng-level-bar>` (`gtk-level-bar`)                       | Meter / Indicator    | `GtkLevelBar`             | `Microsoft.UI.Xaml.Controls.ProgressBar`     | `NSLevelIndicator`    | `value`, `min`, `max`                                                                                                             |
| `<ng-progress-bar>` (`progress`, `gtk-progress-bar`)     | Feedback             | `GtkProgressBar`          | `Microsoft.UI.Xaml.Controls.ProgressBar`     | `NSProgressIndicator` | `value` (`0.0..1.0`), `indeterminate`                                                                                             |
| `<ng-spinner>` (`gtk-spinner`)                           | Feedback             | `GtkSpinner`              | `Microsoft.UI.Xaml.Controls.ProgressRing`    | `NSProgressIndicator` | `active`                                                                                                                          |
| `<ng-button>` (`button`, `gtk-button`)                   | Action Control       | `GtkButton`               | `Microsoft.UI.Xaml.Controls.Button`          | `NSButton`            | label text, `(click)`                                                                                                             |
| `<ng-entry>` (`input`, `gtk-entry`)                      | Form Input           | `GtkEntry`                | `Microsoft.UI.Xaml.Controls.TextBox`         | `NSTextFieldEditable` | `value` / `text`, `placeholder`, `(input)` + `NativeTextValueAccessor`                                                            |
| `<ng-search-entry>` (`gtk-search-entry`)                 | Search Input         | `GtkSearchEntry`          | `Microsoft.UI.Xaml.Controls.AutoSuggestBox`  | `NSSearchField`       | `value` / `text`, `placeholder`, `(input)` + `NativeTextValueAccessor`                                                            |
| `<ng-password-entry>` (`gtk-password-entry`)             | Password Input       | `GtkPasswordEntry`        | `Microsoft.UI.Xaml.Controls.PasswordBox`     | `NSSecureTextField`   | `value` / `text`, `placeholder`, `show-peek-icon` / `peek`, `(input)` + `NativeTextValueAccessor`                                 |
| `<ng-text-view>` (`textarea`, `gtk-text-view`)           | Multiline Text       | `GtkTextView`             | `Microsoft.UI.Xaml.Controls.RichEditBox`     | `NSTextView`          | `value` / `text`, `editable`, `monospace`, `(input)` + `NativeTextValueAccessor`                                                  |
| `<ng-spin-button>` (`gtk-spin-button`)                   | Numeric Stepper      | `GtkSpinButton`           | `Microsoft.UI.Xaml.Controls.NumberBox`       | `NSStepper`           | `value`, `min`, `max`, `step`, `(valueChange)` + `NativeNumberValueAccessor`                                                      |
| `<ng-switch>` (`gtk-switch`)                             | Form Toggle          | `GtkSwitch`               | `Microsoft.UI.Xaml.Controls.ToggleSwitch`    | `NSSwitch`            | `active` / `checked` / `value`, `(change)` + `NativeBooleanValueAccessor`                                                         |
| `<ng-checkbox>` (`ng-check`, `gtk-check-button`)         | Form Toggle          | `GtkCheckButton`          | `Microsoft.UI.Xaml.Controls.CheckBox`        | `NSButtonCheckbox`    | `label`, `active` / `checked` / `value`, `(change)` + `NativeBooleanValueAccessor`                                                |
| `<ng-slider>` (`ng-scale`, `gtk-scale`)                  | Form Range           | `GtkScale`                | `Microsoft.UI.Xaml.Controls.Slider`          | `NSSlider`            | `value`, `min`, `max`, `(valueChange)` + `NativeNumberValueAccessor`                                                              |
| `<ng-select>` (`select`, `ng-dropdown`, `gtk-drop-down`) | Form Select          | `GtkDropDown`             | `Microsoft.UI.Xaml.Controls.ComboBox`        | `NSPopUpButton`       | `options` (CSV), `selected` / `value`, `(valueChange)` + `NativeNumberValueAccessor`                                              |
| `<ng-anchor>` (`#comment`)                               | Virtual Control Flow | `GtkAnchorComment`        | `Microsoft.UI.Xaml.AnchorComment`            | `NSAnchorComment`     | `@if`, `@for`, `@switch`, `@defer`, `<router-outlet />` anchor                                                                    |

> **Common Widget Attributes, Forms & Virtualization:** All widgets support `visible`, `hexpand`, `vexpand`, `halign`, `valign`, `margin*`, `min-width`, `min-height`, and `sensitive`. Interactive inputs support both **Angular Signal Forms (`[formField]` via `NATIVE_SIGNAL_FORM_DIRECTIVES`)** and classic **`ReactiveFormsModule`/`FormsModule` (`NATIVE_FORM_DIRECTIVES`)**. For high-volume lists, `@ng-desktop/platform` also exports standalone `<ng-virtual-list>` (`NativeVirtualListComponent`) backed by `computeVirtualSliceRange`. Import `NATIVE_UI_DIRECTIVES` to use all primitives and mapped HTML tags (`HTML_TO_NATIVE_TAG`) without `CUSTOM_ELEMENTS_SCHEMA`.

---

## 📂 Repository Structure

```text
ng-desktop-engine/
├── src/
│   ├── lib.rs                    # Library entry point & module declarations
│   ├── main.rs                   # Native engine runner, GTK4 event loop & HMR IPC handler
│   ├── config/
│   │   ├── mod.rs                # Angular CLI (angular.json), Nx (project.json), & runtime config resolver
│   │   └── permissions.rs        # Build-time AppPermissions manifest parser & least-privilege serializer
│   ├── desktop/
│   │   ├── mod.rs                # Desktop OS hub (Theme, Clipboard, Notifications, Dialogs, HTTP, SQLite, Env, Compute, FS)
│   │   ├── compute_pool.rs       # Multithreaded Rust compute pool (fuzzy filtering & 64-bit FNV-1a hashing)
│   │   ├── fs_watcher.rs         # XDG Base Directory resolver, atomic file writes & Linux inotify watcher
│   │   ├── http_native.rs        # OS-native HTTP/2 engine (libsoup-3.0 / URLSession / WinHTTP, cache & XSRF)
│   │   ├── http_codec.rs         # JSON bridge serialization & request/response parsing (SRP)
│   │   ├── libsoup_sys.rs        # Isolated C FFI dynamic loader for libsoup-3.0.so.0
│   │   ├── sqlite_sys.rs         # Isolated C FFI dynamic loader for libsqlite3.so.0 (WAL + JSON rows)
│   │   └── system_env.rs         # GNetworkMonitor, GLib locale/timezone, disk image cache & error log
│   ├── ipc/
│   │   ├── mod.rs                # Non-blocking Unix Domain Socket HMR IPC Server & Client
│   │   └── test_harness.rs       # In-process HMR & batch test harness
│   ├── packager/
│   │   ├── mod.rs                # Multi-OS package staging (.deb, AppImage, Flatpak, WinUI AppxManifest, macOS .app)
│   │   ├── builders.rs           # Native .deb, AppImage, Flatpak, MSIX, and .dmg installer compilers
│   │   └── png.rs                # Dependency-free RGBA PNG encoder for app icons & installer assets
│   ├── engine/                   # OS-agnostic core rendering engine (100% safe Rust)
│   │   ├── mod.rs                # RenderEngine<B> scene graph & frame orchestrator
│   │   ├── arena.rs              # Generational Arena + runtime BorrowState checker
│   │   ├── commands.rs           # Frame-batched RenderCommand queue
│   │   ├── default_icon.ico      # Self-contained neutral default window icon asset
│   │   ├── events.rs             # EventMask bitflags, coalescing & typed listener registry
│   │   ├── icons.rs              # Binary .ico multi-resolution parser & WindowIconAsset
│   │   ├── node.rs               # NodeId, StyleId, BorrowState, and EngineError types
│   │   ├── preview.rs            # Scene graph tree inspector & ASCII window previewer
│   │   ├── styles.rs             # Deduplicated StyleRegistry & extended CSS StyleRule
│   │   ├── widget.rs             # Cross-platform semantic widget model (29 primitives + Anchor)
│   │   └── widget_attrs.rs       # Layout & interactive form attribute mutators (SRP)
│   ├── platforms/                # Concrete OS backend adapters (Bridge + Factory)
│   │   ├── mod.rs                # PlatformBackend trait & AnyPlatformBackend factory
│   │   ├── layout_solver.rs      # Shared cross-platform 1D Flexbox & 2D Grid layout solver
│   │   ├── appkit_sys.rs         # Isolated Objective-C runtime FFI loader for macOS AppKit
│   │   ├── gtk4_callbacks.rs     # Thread-safe GTK4 C signal trampolines & lazy EventMask controllers
│   │   ├── gtk4_controls.rs      # Interactive GTK4 control mutators & signal connectors (SRP)
│   │   ├── gtk4_styling.rs       # GTK4 CSS stylesheet builder & widget attribute dispatcher
│   │   ├── gtk4_sys.rs           # Isolated C FFI dynamic loader for libgtk-4.so.1 & libadwaita-1.so.0
│   │   ├── gtk4_widgets.rs       # Safe GTK4 widget & layout container wrappers
│   │   ├── linux_gtk.rs          # Native Linux GTK4 + Freedesktop hicolor icon adapter
│   │   ├── scene_state.rs        # DRY retaining scene state machine (WinUI 3 & AppKit)
│   │   ├── win32_sys.rs          # Isolated Win32 FFI loader (user32.dll / kernel32.dll)
│   │   ├── windows_winui.rs      # Windows WinUI 3 / Win32 adapter
│   │   ├── macos_appkit.rs       # macOS AppKit (Cocoa) adapter
│   │   └── headless.rs           # Headless frame recorder for automated CI tests
│   └── runtime/
│       ├── mod.rs                # Embedded reactive Signal & instruction orchestrator
│       ├── bootstrap_js.rs       # Injected globalThis.__NG_NATIVE__ host bridge runtime
│       ├── host_bridge.rs        # Compiled Angular Ivy runtime host bindings
│       ├── js_bridge.rs          # JS evaluation & native event dispatcher (SRP)
│       ├── js_engine.rs          # Pluggable JsEngineBackend trait & JscEngine adapter
│       ├── jsc_sys.rs            # Isolated C FFI dynamic loader for libjavascriptcoregtk-6.0.so.1
│       ├── quickjs_engine.rs     # Portable zero-JIT QuickJsEngine adapter
│       ├── reactivity.rs         # Signal state propagation & declarative handlers (SRP)
│       ├── scene_exec.rs         # Structural scene instruction executor & handle table (SRP)
│       ├── server_worker.rs      # In-process worker executing @ng-desktop/server route bundles
│       └── parser.rs             # Bytecode (.ngb) instruction IR & syntax parser
├── packages/
│   ├── platform-native/          # @ng-desktop/platform (Renderer2, NATIVE_UI_DIRECTIVES, VirtualList, Ecosystem compat)
│   ├── forms-native/             # @ng-desktop/forms (Signal Forms [formField] controls & ControlValueAccessors)
│   ├── router-native/            # @ng-desktop/router (NativePlatformLocation, NativeTitleStrategy, RouteReuseStrategy)
│   ├── http-native/              # @ng-desktop/http (NativeHttpBackend, provideHttpClient features, createNativeHttpResource)
│   ├── desktop-native/           # @ng-desktop/desktop (Desktop, SQLite, Network, Compute, FileSystem, DropTarget)
│   ├── server-native/            # @ng-desktop/server (defineNativeEventHandler, file-based local routes)
│   └── builders-native/          # @ng-desktop/builders (Angular/Nx resolver, AST permissions, CLI, schematics & HMR)
├── docs/
│   ├── GETTING_STARTED.md        # Step-by-step tutorial for external Angular 22+ & Nx apps
│   └── ECOSYSTEM_COMPATIBILITY.md # Integration guide for NgRx, Angular Aria, CDK, Angular Material & Nx
├── docs-site/                    # Official Angular 22 interactive documentation portal (ng-desktop-docs)
├── docker/
│   ├── Dockerfile.ubuntu-gtk4    # Ubuntu 24.04 container image (GTK4 + Libadwaita + JSC + libsoup3 + SQLite3 + Xvfb)
│   ├── Dockerfile.fedora-gtk4    # Fedora 41 container image (GTK4 + Libadwaita + WebKitGTK 6.0 + libsoup3 + SQLite3 + Xvfb)
│   ├── Dockerfile.windows-wine   # Windows x86_64 cross-compile (MinGW-w64) + Wine64 test runner container
│   ├── Dockerfile.macos-cross    # macOS (aarch64-apple-darwin & x86_64-apple-darwin) cross-target container
│   └── docker-compose.kvm-os.yml # KVM-accelerated Windows 11 & macOS desktop VM containers (dockurr/windows & dockurr/macos)
├── scripts/
│   ├── bootstrap-npm-packages.mjs # One-time npm Trusted Publishing package initializer
│   ├── build-api-docs.mjs         # Unified TypeScript (/api-ts/) & Rust (/api-rust/) API reference generator
│   ├── build-engine-local.mjs     # Local @ng-desktop/engine-* binary stager
│   ├── build-engine-packages.mjs  # Stages @ng-desktop/engine-* platform binary packages
│   ├── build-packages.mjs         # Angular Package Format (APF) compiler (dist/packages/*)
│   ├── generate-release-notes.mjs # Engine & @ng-desktop/* release changelog builder
│   ├── test-containers.sh         # Multi-OS container test runner (ubuntu | fedora | windows | macos | all)
│   └── test-external-pack.mjs     # Verifies npm pack tarballs from an isolated external workspace
├── examples/
│   └── counter-app/              # Angular 22 routed showcase application (features + server/routes)
│       ├── public/               # Application favicon assets
│       └── src/                  # app/, server/routes/, main.ts & styles.css
├── tests/
│   ├── common/mod.rs             # Shared test helpers & fixture loader
│   ├── fixtures/sample-app/      # Self-contained neutral Angular/NGB integration test fixture
│   ├── test_angular_ivy_runtime.rs # Compiled Ivy runtime bridge integration tests
│   ├── test_cross_platform.rs    # Win32/WinUI, AppKit, Linux GTK4 & packager tests
│   ├── test_e2e_harness.rs       # End-to-end sample-app bundle execution tests
│   ├── test_forms_and_desktop.rs # Forms, SQLite, compute, FS, HTTP & desktop bridge tests
│   ├── test_js_engines.rs        # Dual JSC & QuickJS engine parity tests
│   ├── test_native_server_worker.rs # In-process @ng-desktop/server route worker tests
│   ├── test_pipeline_and_hmr.rs  # Full bundle pipeline, HMR & cross-OS socket/TCP IPC tests
│   └── test_scene_graph.rs       # Arena invariants, anchors & reverse event dispatch tests
├── ROADMAP.md                    # Current v0.6.0 snapshot, parity matrix & upcoming milestones
├── CHANGELOG.md                  # Release notes and version changelog
├── SECURITY.md                   # Vulnerability disclosure policy, SLAs & FFI threat model
├── CONTRIBUTING.md               # Contribution guidelines, SOLID standards & release workflow
├── CODE_OF_CONDUCT.md            # Contributor Covenant v2.1 community standards
├── SUPPORT.md                    # Documentation links & where to file bugs/features
├── LICENSE                       # MIT License
└── .github/
    ├── ISSUE_TEMPLATE/           # Bug, Feature Request, and Cross-Platform Parity forms
    ├── PULL_REQUEST_TEMPLATE.md  # SOLID, FFI safety, and multi-OS verification checklist
    ├── CODEOWNERS                # Repository review ownership
    ├── dependabot.yml            # Weekly Cargo, npm, and GitHub Actions security updates
    └── workflows/                # Multi-OS CI (Linux/Win/macOS) & consolidated OIDC Release workflow
```

---

## ⚡ Quick Start

### 1. Using `@ng-desktop/*` in an External Angular 22+ or Nx App

Follow the complete guide in [**docs/GETTING_STARTED.md**](./docs/GETTING_STARTED.md), or bootstrap directly from your Angular workspace:

```bash
ng add @ng-desktop/builders
npx ng-desktop permissions
npx ng-desktop build
npx ng-desktop run
```

### 2. Developing Inside This Monorepo

#### Prerequisites

- **Rust** (stable toolchain, 2024 edition)
- **Node.js** `>= 24` & **pnpm** `>= 11`
- **Linux Desktop (for live GUI & full FFI):** `libgtk-4-1`, `libadwaita-1-0`, `libjavascriptcoregtk-6.0-1`, `libsoup-3.0-0`, `libsqlite3-0`.

#### Build & Run

```bash
# 1. Install Angular & tooling dependencies
pnpm install

# 2. Compile Angular 22 bundle, .ngb IR, and server routes
pnpm run build

# 3. Live development with zero-latency Unix socket HMR
pnpm run dev

# 4. Stage multi-OS bundles or build real native installers (.deb / .msix / .dmg)
pnpm run package:linux   # or: pnpm run package:all / npx ng-desktop package --installer=deb --strict-installer

# 5. Launch native window directly (or non-blocking CI batch mode)
cargo run                # or: NG_NATIVE_BATCH=1 cargo run
```

---

## 🛠️ Quality & Verification Commands

| Command                                             | Description                                                              |
| :-------------------------------------------------- | :----------------------------------------------------------------------- |
| `pnpm run dev`                                      | Watch Angular files & hot-reload native GTK4 window via Unix socket      |
| `pnpm run build:packages`                           | Compile all 7 `@ng-desktop/*` packages into `dist/packages/*`            |
| `pnpm run build:engine-packages -- --all-manifests` | Stage `@ng-desktop/engine-*` optional binary packages                    |
| `pnpm run test:pack`                                | Pack `.tgz` archives and validate external out-of-monorepo consumption   |
| `pnpm run check`                                    | Run full verification gate (OxFmt + OxLint + ESLint + Clippy + Test)     |
| `pnpm run package:all`                              | Build bundle and stage Linux, Windows, and macOS distribution packages   |
| `pnpm run format:all`                               | Format both TypeScript/Angular/Markdown (`oxfmt`) and Rust (`cargo fmt`) |
| `pnpm run format:check`                             | Verify formatting with `oxfmt`                                           |
| `pnpm run lint`                                     | Lint TypeScript & Angular templates via `oxlint` + `angular-eslint`      |
| `pnpm run test --watch=false`                       | Run all 102 Vitest specs across 17 test files (`ng-linux-counter`)       |
| `pnpm run docs:test`                                | Run Docs Portal & API pipeline Vitest specs (`ng-desktop-docs`)          |
| `pnpm run docs:site`                                | Build Docs Portal + `/api-ts/` (285 symbols) + `/api-rust/` rustdoc      |
| `pnpm run test:containers`                          | Run Xvfb + GTK4 + FFI test suite in Ubuntu 24.04 & Fedora 41 containers  |
| `cargo fmt --all -- --check`                        | Verify Rust formatting                                                   |
| `cargo clippy --all-targets -- -D warnings`         | Run strict Rust Clippy lints (pedantic + nursery, zero warnings)         |
| `cargo test`                                        | Execute all 90 unit tests + 46 integration tests (8 suites) in Rust      |

---

## 🗺️ Documentation, Roadmap & Ecosystem Compatibility

- See [**GETTING_STARTED.md**](./docs/GETTING_STARTED.md) for a copy-paste tutorial integrating `@ng-desktop/*` into an external Angular 22+ or Nx app.
- See [**ECOSYSTEM_COMPATIBILITY.md**](./docs/ECOSYSTEM_COMPATIBILITY.md) for integration with **NgRx**, **Angular Aria**, **Angular CDK**, **Angular Material** gap analysis, **@ng-desktop/server**, and **Nx**.
- See [**ROADMAP.md**](./ROADMAP.md) for the `v0.6.0` platform maturity matrix, remaining gaps, and upcoming `v0.7.0` milestones.
- See [**CHANGELOG.md**](./CHANGELOG.md) for release notes and detailed version history.

---

## 🤝 Contributing, Security & Governance

We welcome contributions across Rust core, platform backends (`GTK4`, `WinUI 3`, `AppKit`), Angular 22+ packages (`@ng-desktop/*`), and packaging/containers:

- **Contributing Guidelines:** Read [**CONTRIBUTING.md**](./CONTRIBUTING.md) for setup, SOLID/FFI standards, and verification commands (`pnpm run check`).
- **Security Policy & Private Disclosure:** See [**SECURITY.md**](./SECURITY.md) — **never report security vulnerabilities publicly**; use [GitHub Private Vulnerability Reporting](https://github.com/Gaspar1992/ng-desktop-engine/security/advisories/new).
- **Reporting Bugs & Feature Proposals:** Use the structured forms in [GitHub Issues](https://github.com/Gaspar1992/ng-desktop-engine/issues/new/choose) (Bug Report, Feature Request, or Cross-Platform Parity).
- **Community & Support:** Reviewed under the [**Contributor Covenant Code of Conduct**](./CODE_OF_CONDUCT.md). See [**SUPPORT.md**](./SUPPORT.md) for navigation and [**LICENSE**](./LICENSE) (MIT).
