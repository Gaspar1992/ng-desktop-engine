# 🚀 NgDesktop Engine (`ng-desktop-engine`)

A high-performance, memory-safe **Native Desktop Rendering Engine for Angular 22+**, written in **Rust**.

`ng-desktop-engine` replaces `@angular/platform-browser` and the browser DOM/WebView completely. Instead of bundling Chromium (Electron) or relying on an embedded HTML WebView (Tauri), Angular components drive **100% native OS widgets** — starting with **Linux (GTK4 / Libadwaita)** and architected via the **Bridge**, **State**, and **Abstract Factory** patterns to target **Windows (WinUI 3)** and **macOS (AppKit / Cocoa)**.

---

## 🎯 Architectural Principles

1. **Zero WebView Overhead:** Angular templates compile and execute against native operating system controls (`AdwApplicationWindow`, `GtkHeaderBar`, `AdwClamp`, `GtkBox`, `GtkGrid`, `GtkFlowBox`, `GtkPaned`, `GtkStack`, `GtkExpander`, `GtkScrolledWindow`, `GtkLabel`, `GtkPicture`, `GtkLevelBar`, `GtkProgressBar`, `GtkSpinner`, `GtkButton`, `GtkEntry`, `GtkSearchEntry`, `GtkPasswordEntry`, `GtkTextView`, `GtkSpinButton`, `GtkSwitch`, `GtkCheckButton`, `GtkScale`, `GtkDropDown`), delivering sub-second startup and a tiny memory footprint.
2. **Rust-Grade Memory Safety Across the JS/Native Boundary:**
   - **Generational Arena (`Arena` + `NodeId`):** JavaScript never holds raw C/GTK pointers. Every native widget is stored inside a Rust generational arena (`index` + `generation`), preventing use-after-free bugs when components unmount.
   - **Runtime Borrow Checker (`BorrowState`):** Enforces Rust's aliasing rules (`Shared(usize)` vs `Exclusive`) at runtime across asynchronous UI events, blocking illegal node destruction while an event callback is actively borrowing a widget.
3. **Game-Engine Inspired Render Pipeline (`CommandQueue`):**
   - Angular mutations during a change-detection tick are batched into an OS-agnostic [`RenderCommand`](./src/engine/commands.rs) buffer (`CreateWidget`, `CreateComment`, `SetText`, `SetAttribute`, `SetWindowIcon`, `RegisterStyle`, `ApplyStyleClass`, `AppendChild`, `InsertBefore`, `RemoveChild`, `DestroyWidget`) and flushed atomically per frame.
4. **AST-Driven Compilation Pipeline (Zero Regex Parsing):**
   - **TypeScript 6.0 AST:** Powered by Rust-based [`oxc-parser`](./packages/builders-native/src/ast-metadata-extractor.mjs), extracting component decorators, `signal()`, `computed()`, `linkedSignal()`, `input()`, `model()`, `output()`, `httpResource()`, and reactive mutation methods.
   - **Angular Template AST:** Powered by official [`@angular/compiler`](./packages/builders-native/src/template-compiler.mjs) `parseTemplate`, compiling control flow (`@if`/`@else`, `@for`/`@empty`, `@switch`, `@let`, `@defer`/`@error`), Angular pipes, dynamic inputs, reactive `bind_visible`/`bind_attr` bindings, and events.
   - **Native CSS AST:** Powered by [`postcss`](./packages/builders-native/src/style-compiler.mjs), translating standard CSS (`padding`, `gap`, `border-radius`, `font-size`, `font-weight`) into native GTK4 / WinUI / AppKit styling.
5. **Real-Time Unix Socket HMR IPC:**
   - Non-blocking IPC server ([`src/ipc/`](./src/ipc/mod.rs)) communicating over local Unix Domain Sockets (`dist/<project>/hmr.sock`) with a Node.js client ([`hmr-client.mjs`](./packages/builders-native/src/hmr-client.mjs)) for zero-latency, atomic hot module reloads while preserving active signal state.
6. **SOLID & Hexagonal Platform Architecture (`PlatformBackend`):**
   - The core [`RenderEngine<B: PlatformBackend>`](./src/engine/mod.rs) depends exclusively on an abstract trait, never on concrete OS toolkits.
   - Concrete platform adapters live in [`src/platforms/`](./src/platforms/):
     - **Linux GTK4 (`GtkLinuxBackend`):** Dynamically loads `libgtk-4.so.1`, `libgobject-2.0.so.0`, `libgio-2.0.so.0`, and `libglib-2.0.so.0` via an isolated FFI boundary ([`gtk4_sys.rs`](./src/platforms/gtk4_sys.rs)).
     - **Windows WinUI 3 (`WinUiBackend`):** Maps abstract primitives to `Microsoft.UI.Xaml` controls via retained [`NativeSceneState`](./src/platforms/scene_state.rs).
     - **macOS AppKit (`MacOsAppKitBackend`):** Maps abstract primitives to Cocoa `NSWindow` / `NSView` controls via retained [`NativeSceneState`](./src/platforms/scene_state.rs).
     - **Headless CI (`HeadlessBackend`):** Records every frame's `RenderCommand` batch in memory for deterministic integration testing in CI without a display server.

---

## 🧩 Native Primitives Catalog

### Implemented Native Primitives (27 Primitives)

| Tag (Aliases)                                          | Category             | Linux (GTK4 / Libadwaita) | Windows (WinUI 3)                            | macOS (AppKit)        | Attributes & Events                                                                                                               |
| :----------------------------------------------------- | :------------------- | :------------------------ | :------------------------------------------- | :-------------------- | :-------------------------------------------------------------------------------------------------------------------------------- |
| `<ng-window>` (`adw-window`)                           | Window               | `AdwApplicationWindow`    | `Microsoft.UI.Xaml.Window`                   | `NSWindow`            | `title`, window icon                                                                                                              |
| `<ng-header-bar>` (`adw-header-bar`, `gtk-header-bar`) | Window Chrome        | `AdwHeaderBar`            | `Microsoft.UI.Xaml.Controls.TitleBar`        | `NSToolbar`           | `title`, `subtitle`, `show-title-buttons`                                                                                         |
| `<ng-clamp>` (`adw-clamp`)                             | Adaptive Container   | `AdwClamp`                | `Microsoft.UI.Xaml.Controls.Grid`            | `NSBox`               | `max-size` / `maximum-size` (px), single centered child                                                                           |
| `<ng-box>` (`gtk-box`)                                 | Layout Container     | `GtkBox`                  | `Microsoft.UI.Xaml.Controls.StackPanel`      | `NSStackView`         | `orientation` (`vertical` \| `horizontal`), `spacing` / `gap`                                                                     |
| `<ng-grid>` (`gtk-grid`)                               | Layout Container     | `GtkGrid`                 | `Microsoft.UI.Xaml.Controls.Grid`            | `NSGridView`          | `spacing` / `gap`, `row-spacing`, `column-spacing`                                                                                |
| `<ng-flow>` (`ng-flow-box`, `gtk-flow-box`)            | Responsive Layout    | `GtkFlowBox`              | `Microsoft.UI.Xaml.Controls.ItemsRepeater`   | `NSCollectionView`    | `spacing` / `gap`, `row-spacing`, `column-spacing`, `min-children` / `min-per-row`, `max-children` / `max-per-row`, `homogeneous` |
| `<ng-paned>` (`gtk-paned`)                             | Split Container      | `GtkPaned`                | `Microsoft.UI.Xaml.Controls.SplitView`       | `NSSplitView`         | `orientation` (`horizontal` \| `vertical`), `position` (px), start/end children                                                   |
| `<ng-stack>` (`gtk-stack`, `adw-navigation-view`)      | Multi-Page Container | `GtkStack`                | `Microsoft.UI.Xaml.Controls.NavigationView`  | `NSTabView`           | `visible-child` / `active-page` (`page_0`...), `transition` / `transition-type`                                                   |
| `<ng-expander>` (`gtk-expander`)                       | Disclosure Container | `GtkExpander`             | `Microsoft.UI.Xaml.Controls.Expander`        | `NSDisclosureButton`  | `label` / `title`, `expanded` / `open`, `notify::expanded`                                                                        |
| `<ng-scroll>` (`gtk-scrolled-window`)                  | Scroll Container     | `GtkScrolledWindow`       | `Microsoft.UI.Xaml.Controls.ScrollViewer`    | `NSScrollView`        | `min-height`                                                                                                                      |
| `<ng-separator>` (`gtk-separator`)                     | Layout Divider       | `GtkSeparator`            | `Microsoft.UI.Xaml.Controls.AppBarSeparator` | `NSBoxSeparator`      | `orientation` (`horizontal` \| `vertical`)                                                                                        |
| `<ng-label>` (`gtk-label`)                             | Typography           | `GtkLabel`                | `Microsoft.UI.Xaml.Controls.TextBlock`       | `NSTextField`         | text content / `{{ signal() }}`                                                                                                   |
| `<ng-image>` (`ng-picture`, `gtk-picture`)             | Media                | `GtkPicture`              | `Microsoft.UI.Xaml.Controls.Image`           | `NSImageView`         | `src`, `width`, `height`                                                                                                          |
| `<ng-level-bar>` (`gtk-level-bar`)                     | Meter / Indicator    | `GtkLevelBar`             | `Microsoft.UI.Xaml.Controls.ProgressBar`     | `NSLevelIndicator`    | `value`, `min`, `max`                                                                                                             |
| `<ng-progress-bar>` (`gtk-progress-bar`)               | Feedback             | `GtkProgressBar`          | `Microsoft.UI.Xaml.Controls.ProgressBar`     | `NSProgressIndicator` | `value` (`0.0..1.0`), `indeterminate`                                                                                             |
| `<ng-spinner>` (`gtk-spinner`)                         | Feedback             | `GtkSpinner`              | `Microsoft.UI.Xaml.Controls.ProgressRing`    | `NSProgressIndicator` | `active`                                                                                                                          |
| `<ng-button>` (`gtk-button`)                           | Action Control       | `GtkButton`               | `Microsoft.UI.Xaml.Controls.Button`          | `NSButton`            | label text, `(click)`                                                                                                             |
| `<ng-entry>` (`gtk-entry`)                             | Form Input           | `GtkEntry`                | `Microsoft.UI.Xaml.Controls.TextBox`         | `NSTextFieldEditable` | `value` / `text`, `placeholder`, `(input)` + `NativeTextValueAccessor`                                                            |
| `<ng-search-entry>` (`gtk-search-entry`)               | Search Input         | `GtkSearchEntry`          | `Microsoft.UI.Xaml.Controls.AutoSuggestBox`  | `NSSearchField`       | `value` / `text`, `placeholder`, `(input)` + `NativeTextValueAccessor`                                                            |
| `<ng-password-entry>` (`gtk-password-entry`)           | Password Input       | `GtkPasswordEntry`        | `Microsoft.UI.Xaml.Controls.PasswordBox`     | `NSSecureTextField`   | `value` / `text`, `placeholder`, `show-peek-icon` / `peek`, `(input)` + `NativeTextValueAccessor`                                 |
| `<ng-text-view>` (`gtk-text-view`)                     | Multiline Text       | `GtkTextView`             | `Microsoft.UI.Xaml.Controls.RichEditBox`     | `NSTextView`          | `value` / `text`, `editable`, `monospace`, `(input)` + `NativeTextValueAccessor`                                                  |
| `<ng-spin-button>` (`gtk-spin-button`)                 | Numeric Stepper      | `GtkSpinButton`           | `Microsoft.UI.Xaml.Controls.NumberBox`       | `NSStepper`           | `value`, `min`, `max`, `step`, `(valueChange)` + `NativeNumberValueAccessor`                                                      |
| `<ng-switch>` (`gtk-switch`)                           | Form Toggle          | `GtkSwitch`               | `Microsoft.UI.Xaml.Controls.ToggleSwitch`    | `NSSwitch`            | `active` / `checked` / `value`, `(change)` + `NativeBooleanValueAccessor`                                                         |
| `<ng-checkbox>` (`ng-check`, `gtk-check-button`)       | Form Toggle          | `GtkCheckButton`          | `Microsoft.UI.Xaml.Controls.CheckBox`        | `NSButtonCheckbox`    | `label`, `active` / `checked` / `value`, `(change)` + `NativeBooleanValueAccessor`                                                |
| `<ng-slider>` (`ng-scale`, `gtk-scale`)                | Form Range           | `GtkScale`                | `Microsoft.UI.Xaml.Controls.Slider`          | `NSSlider`            | `value`, `min`, `max`, `(valueChange)` + `NativeNumberValueAccessor`                                                              |
| `<ng-select>` (`ng-dropdown`, `gtk-drop-down`)         | Form Select          | `GtkDropDown`             | `Microsoft.UI.Xaml.Controls.ComboBox`        | `NSPopUpButton`       | `options` (CSV), `selected` / `value`, `(valueChange)` + `NativeNumberValueAccessor`                                              |
| `<ng-anchor>` (`#comment`)                             | Virtual Control Flow | `GtkAnchorComment`        | `Microsoft.UI.Xaml.AnchorComment`            | `NSAnchorComment`     | `@if`, `@for`, `@switch` anchor                                                                                                   |

> **Common Widget Attributes:** `visible`, `hexpand`, `vexpand`, `halign`, `valign`, `margin`, `margin-top`, `margin-bottom`, `margin-start`, `margin-end` (plus `sensitive` emitted by `@ng-native/forms` `setDisabledState`).

---

## 📂 Repository Structure

```text
ng-native-engine/
├── src/
│   ├── lib.rs                    # Library entry point & module declarations
│   ├── main.rs                   # Native engine runner, GTK4 event loop & HMR IPC handler
│   ├── config/
│   │   └── mod.rs                # Angular CLI (angular.json) native target parser
│   ├── desktop/
│   │   ├── mod.rs                # Desktop OS hub (Theme, Clipboard, Notifications, Dialogs, HTTP, SQLite, Env, Compute, FS)
│   │   ├── compute_pool.rs       # Multithreaded Rust compute pool (fuzzy filtering & 64-bit FNV-1a hashing)
│   │   ├── fs_watcher.rs         # XDG Base Directory resolver, atomic file writes & Linux inotify watcher
│   │   ├── http_native.rs        # OS-native HTTP/2 engine (libsoup-3.0 / URLSession / WinHTTP, cache & XSRF)
│   │   ├── http_codec.rs         # JSON bridge serialization & request/response parsing (SRP)
│   │   ├── libsoup_sys.rs        # Isolated C FFI dynamic loader for libsoup-3.0.so.0
│   │   ├── sqlite_sys.rs         # Isolated C FFI dynamic loader for libsqlite3.so.0 (WAL + JSON rows)
│   │   ├── system_env.rs         # GNetworkMonitor, GLib locale/timezone, disk image cache & error log
│   │   ├── pokedex.rs            # Live PokéAPI HTTP client & sprite cache
│   │   └── pokedex_ui.rs         # Async Pokédex controller, debounced search & loading spinner
│   ├── ipc/
│   │   └── mod.rs                # Non-blocking Unix Domain Socket HMR IPC Server & Client
│   ├── packager/
│   │   └── mod.rs                # Linux distribution bundler (.deb, AppImage, Flatpak, .desktop)
│   ├── engine/                   # OS-agnostic core rendering engine (100% safe Rust)
│   │   ├── mod.rs                # RenderEngine<B> scene graph & frame orchestrator
│   │   ├── arena.rs              # Generational Arena + runtime BorrowState checker
│   │   ├── commands.rs           # Frame-batched RenderCommand queue
│   │   ├── events.rs             # Native-to-Angular typed event listener registry
│   │   ├── icons.rs              # Binary .ico multi-resolution parser & WindowIconAsset
│   │   ├── node.rs               # NodeId, StyleId, BorrowState, and EngineError types
│   │   ├── preview.rs            # Scene graph tree inspector & ASCII window previewer
│   │   ├── styles.rs             # Deduplicated StyleRegistry & extended CSS StyleRule
│   │   ├── widget.rs             # Cross-platform semantic widget model
│   │   └── widget_attrs.rs       # Layout & interactive form attribute mutators (SRP)
│   ├── platforms/                # Concrete OS backend adapters (Bridge + Factory)
│   │   ├── mod.rs                # PlatformBackend trait & AnyPlatformBackend factory
│   │   ├── gtk4_callbacks.rs     # Thread-safe GTK4 C signal trampolines & event queue
│   │   ├── gtk4_controls.rs      # Interactive GTK4 control mutators & signal connectors (SRP)
│   │   ├── gtk4_styling.rs       # GTK4 CSS stylesheet builder & widget attribute dispatcher
│   │   ├── gtk4_sys.rs           # Isolated C FFI dynamic loader for libgtk-4.so.1
│   │   ├── gtk4_widgets.rs       # Safe GTK4 widget & layout container wrappers
│   │   ├── linux_gtk.rs          # Native Linux GTK4 + Freedesktop hicolor icon adapter
│   │   ├── scene_state.rs        # DRY retaining scene state machine (WinUI 3 & AppKit)
│   │   ├── windows_winui.rs      # Windows WinUI 3 adapter
│   │   ├── macos_appkit.rs       # macOS AppKit (Cocoa) adapter
│   │   └── headless.rs           # Headless frame recorder for automated CI tests
│   └── runtime/
│       ├── mod.rs                # Embedded reactive Signal & instruction orchestrator
│       ├── bootstrap_js.rs       # Injected globalThis.__NG_NATIVE__ host bridge runtime
│       ├── js_bridge.rs          # JS evaluation & native event dispatcher (SRP)
│       ├── js_engine.rs          # Pluggable JsEngineBackend trait & JscEngine adapter
│       ├── jsc_sys.rs            # Isolated C FFI dynamic loader for libjavascriptcoregtk-6.0.so.1
│       ├── quickjs_engine.rs     # Portable zero-JIT QuickJsEngine adapter
│       ├── reactivity.rs         # Signal state propagation & declarative handlers (SRP)
│       ├── scene_exec.rs         # Structural scene instruction executor & handle table (SRP)
│       └── parser.rs             # Bytecode (.ngb) instruction IR & syntax parser
├── packages/
│   ├── platform-native/          # @ng-native/platform (Renderer2, IMAGE_LOADER, Title, ViewportScroller, Locale, KeyEvents, ErrorHandler)
│   ├── forms-native/             # @ng-native/forms ControlValueAccessor directives
│   ├── router-native/            # @ng-native/router (NativePlatformLocation, NativeTitleStrategy, NativeStackRouteReuseStrategy)
│   ├── http-native/              # @ng-native/http (NativeHttpBackend, provideHttpClient features, createNativeHttpResource)
│   ├── desktop-native/           # @ng-native/desktop (Desktop, SQLite, Network, Compute, FileSystem, DropTarget)
│   └── builders-native/          # @ng-native/builders (Angular 22 Architect builder, schema, AST compilers, HMR & dev server)
├── docker/
│   ├── Dockerfile.ubuntu-gtk4    # Ubuntu 24.04 container image (GTK4 + Libadwaita + JSC + libsoup3 + SQLite3 + Xvfb)
│   ├── Dockerfile.fedora-gtk4    # Fedora 41 container image (GTK4 + Libadwaita + WebKitGTK 6.0 + libsoup3 + SQLite3 + Xvfb)
│   ├── Dockerfile.windows-wine   # Windows x86_64 cross-compile (MinGW-w64) + Wine64 test runner container
│   ├── Dockerfile.macos-cross    # macOS (aarch64-apple-darwin & x86_64-apple-darwin) cross-target container
│   └── docker-compose.kvm-os.yml # KVM-accelerated Windows 11 & macOS desktop VM containers (dockurr/windows & dockurr/macos)
├── scripts/
│   └── test-containers.sh        # Multi-OS container test runner (ubuntu | fedora | windows | macos | all)
├── examples/
│   └── counter-app/              # Pure Angular 22 Pokédex showcase application
│       ├── public/               # Application favicon assets
│       └── src/app/              # app.{ts,html,css,config,spec}, pokedex.service.ts, pokemon-card.*, type-badge.*
├── tests/
│   ├── common/mod.rs             # Shared test helpers & headless harness
│   ├── test_cross_platform.rs    # WinUI 3, AppKit, Linux GTK4 & packager tests
│   ├── test_forms_and_desktop.rs # Phase 4 forms & Phase 5 desktop/HTTP bridge tests
│   ├── test_js_engines.rs        # Dual JSC & QuickJS engine parity tests
│   ├── test_pipeline_and_hmr.rs  # Full bundle pipeline, HMR & Unix socket IPC tests
│   └── test_scene_graph.rs       # Arena invariants, anchors & reverse event dispatch tests
├── ROADMAP.md                    # Deep architectural roadmap & completed milestone history
└── CHANGELOG.md                  # Release notes and version changelog
```

---

## ⚡ Quick Start

### Prerequisites

- **Rust** (stable toolchain, 2024 edition)
- **Node.js** `>= 24` & **pnpm** `>= 11`
- **Linux Desktop (optional for live GUI):** `libgtk-4-1` runtime library (`libgtk-4.so.1` — pre-installed on modern Ubuntu/Debian/Fedora/Arch desktops; no `-dev` C headers required).

### 1. Install Angular & Tooling Dependencies

```bash
pnpm install
```

### 2. Compile the Angular 22 Project & Native Bundle (`.ngb`)

```bash
pnpm run build
```

This runs the official Angular 22 compiler (`ng build`) followed by the AST-driven native compiler ([`build-bundle.mjs`](./packages/builders-native/src/build-bundle.mjs)), generating:

- `dist/ng-linux-counter/main.ngb` (compiled native instructions, styles, and reactive signal bindings)
- `dist/ng-linux-counter/icons/hicolor/<size>x<size>/apps/ng-linux-counter.png` (`16x16` through `256x256` taskbar icons)

### 3. Live Development with Zero-Latency Socket HMR (`pnpm run dev`)

```bash
pnpm run dev
```

Starts the file watcher on `examples/counter-app/src/` and opens the native Linux GTK4 window. Whenever you save `app.ts`, `app.html`, or `app.css`, `build-bundle.mjs` recompiles and immediately pushes the update over the local Unix domain socket IPC to the running engine **in-place without closing the window or losing current Angular Signal state** (`count`).

### 4. Stage Linux Distribution Packages (`.deb`, `AppImage`, `Flatpak`)

```bash
pnpm run package:linux
```

Executes the Angular builder and the Rust distribution packager to produce installable Linux packages with desktop entries and hicolor icons.

### 5. Launch the Native Linux GTK4 Window Directly

```bash
cargo run
```

To run in non-blocking batch mode (e.g., in terminal/CI scripts):

```bash
NG_NATIVE_BATCH=1 cargo run
```

---

## 🛠️ Quality & Verification Commands

| Command                                     | Description                                                             |
| :------------------------------------------ | :---------------------------------------------------------------------- |
| `pnpm run dev`                              | Watch Angular files & hot-reload native GTK4 window via Unix socket     |
| `pnpm run check`                            | Run full verification gate (OxFmt + OxLint + ESLint + Clippy + Test)    |
| `pnpm run package:linux`                    | Build bundle and stage Linux `.deb`, AppImage, and Flatpak packages     |
| `pnpm run format:all`                       | Format both TypeScript/Angular (`oxfmt`) and Rust (`cargo fmt`)         |
| `pnpm run format`                           | Format TypeScript, HTML, CSS, JSON, and Markdown via `oxfmt`            |
| `pnpm run format:check`                     | Verify formatting with `oxfmt`                                          |
| `pnpm run lint`                             | Lint TypeScript & Angular templates via `oxlint` + `angular-eslint`     |
| `pnpm run test`                             | Run all 43 Vitest specs for packages and showcase app                   |
| `pnpm run test:containers`                  | Run Xvfb + GTK4 + FFI test suite in Ubuntu 24.04 & Fedora 41 containers |
| `cargo fmt --all -- --check`                | Verify Rust formatting                                                  |
| `cargo clippy --all-targets -- -D warnings` | Run strict Rust Clippy lints (pedantic + nursery, zero warnings)        |
| `cargo test`                                | Execute all 40 unit tests + 19 E2E integration tests in Rust            |

---

## 🗺️ Roadmap & Changelog

- See [**ROADMAP.md**](./ROADMAP.md) for the complete technical analysis and phased milestones to reach **100% Angular ecosystem compatibility** on Linux, Windows, and macOS.
- See [**CHANGELOG.md**](./CHANGELOG.md) for release notes and detailed version history.
