# 🚀 NgNative Engine (`ng-native-engine`)

A high-performance, memory-safe **Native Desktop Rendering Engine for Angular 22**, written in **Rust**.

`ng-native-engine` replaces `@angular/platform-browser` and the browser DOM/WebView completely. Instead of bundling Chromium (Electron) or relying on an embedded HTML WebView (Tauri), Angular components drive **100% native OS widgets** — starting with **Linux (GTK4 / Libadwaita)** and architected via the **Bridge**, **State**, and **Abstract Factory** patterns to target **Windows (WinUI 3)** and **macOS (AppKit / Cocoa)**.

---

## 🎯 Architectural Principles

1. **Zero WebView Overhead:** Angular templates compile and execute against native operating system controls (`AdwApplicationWindow`, `GtkHeaderBar`, `GtkBox`, `GtkGrid`, `GtkFlowBox`, `GtkScrolledWindow`, `GtkLabel`, `GtkPicture`, `GtkLevelBar`, `GtkProgressBar`, `GtkSpinner`, `GtkButton`, `GtkEntry`, `GtkSwitch`, `GtkCheckButton`, `GtkScale`, `GtkDropDown`), delivering sub-second startup and a tiny memory footprint.
2. **Rust-Grade Memory Safety Across the JS/Native Boundary:**
   - **Generational Arena (`Arena` + `NodeId`):** JavaScript never holds raw C/GTK pointers. Every native widget is stored inside a Rust generational arena (`index` + `generation`), preventing use-after-free bugs when components unmount.
   - **Runtime Borrow Checker (`BorrowState`):** Enforces Rust's aliasing rules (`Shared(usize)` vs `Exclusive`) at runtime across asynchronous UI events, blocking illegal node destruction while an event callback is actively borrowing a widget.
3. **Game-Engine Inspired Render Pipeline (`CommandQueue`):**
   - Angular mutations during a change-detection tick are batched into an OS-agnostic [`RenderCommand`](./src/engine/commands.rs) buffer (`CreateWidget`, `CreateComment`, `SetText`, `SetAttribute`, `SetWindowIcon`, `RegisterStyle`, `ApplyStyleClass`, `AppendChild`, `InsertBefore`, `RemoveChild`, `DestroyWidget`) and flushed atomically per frame.
4. **AST-Driven Compilation Pipeline (Zero Regex Parsing):**
   - **TypeScript 6.0 AST:** Powered by Rust-based [`oxc-parser`](./examples/counter-app/ast-metadata-extractor.mjs), extracting component decorators, `signal()` declarations, and reactive mutation methods.
   - **Angular Template AST:** Powered by official [`@angular/compiler`](./examples/counter-app/template-compiler.mjs) `parseTemplate`, compiling control flow (`@if`), dynamic inputs, reactive text bindings, and events.
   - **Native CSS AST:** Powered by [`postcss`](./examples/counter-app/style-compiler.mjs), translating standard CSS (`padding`, `gap`, `border-radius`, `font-size`, `font-weight`) into native GTK4 / WinUI / AppKit styling.
5. **Real-Time Unix Socket HMR IPC:**
   - Non-blocking IPC server ([`src/ipc/`](./src/ipc/mod.rs)) communicating over local Unix Domain Sockets (`dist/<project>/hmr.sock`) with a Node.js client ([`hmr-client.mjs`](./examples/counter-app/hmr-client.mjs)) for zero-latency, atomic hot module reloads while preserving active signal state.
6. **SOLID & Hexagonal Platform Architecture (`PlatformBackend`):**
   - The core [`RenderEngine<B: PlatformBackend>`](./src/engine/mod.rs) depends exclusively on an abstract trait, never on concrete OS toolkits.
   - Concrete platform adapters live in [`src/platforms/`](./src/platforms/):
     - **Linux GTK4 (`GtkLinuxBackend`):** Dynamically loads `libgtk-4.so.1`, `libgobject-2.0.so.0`, `libgio-2.0.so.0`, and `libglib-2.0.so.0` via an isolated FFI boundary ([`gtk4_sys.rs`](./src/platforms/gtk4_sys.rs)).
     - **Windows WinUI 3 (`WinUiBackend`):** Maps abstract primitives to `Microsoft.UI.Xaml` controls via retained [`NativeSceneState`](./src/platforms/scene_state.rs).
     - **macOS AppKit (`MacOsAppKitBackend`):** Maps abstract primitives to Cocoa `NSWindow` / `NSView` controls via retained [`NativeSceneState`](./src/platforms/scene_state.rs).
     - **Headless CI (`HeadlessBackend`):** Records every frame's `RenderCommand` batch in memory for deterministic integration testing in CI without a display server.

---

## 🧩 Native Primitives Catalog

### Implemented Native Primitives (19 Primitives)

| Tag (Aliases)                                          | Category             | Linux (GTK4 / Libadwaita) | Windows (WinUI 3)                            | macOS (AppKit)        | Attributes & Events                                                                                                               |
| :----------------------------------------------------- | :------------------- | :------------------------ | :------------------------------------------- | :-------------------- | :-------------------------------------------------------------------------------------------------------------------------------- |
| `<ng-window>` (`adw-window`)                           | Window               | `AdwApplicationWindow`    | `Microsoft.UI.Xaml.Window`                   | `NSWindow`            | `title`, window icon                                                                                                              |
| `<ng-header-bar>` (`adw-header-bar`, `gtk-header-bar`) | Window Chrome        | `AdwHeaderBar`            | `Microsoft.UI.Xaml.Controls.TitleBar`        | `NSToolbar`           | `title`, `subtitle`, `show-title-buttons`                                                                                         |
| `<ng-box>` (`gtk-box`)                                 | Layout Container     | `GtkBox`                  | `Microsoft.UI.Xaml.Controls.StackPanel`      | `NSStackView`         | `orientation` (`vertical` \| `horizontal`), `spacing` / `gap`                                                                     |
| `<ng-grid>` (`gtk-grid`)                               | Layout Container     | `GtkGrid`                 | `Microsoft.UI.Xaml.Controls.Grid`            | `NSGridView`          | `spacing` / `gap`, `row-spacing`, `column-spacing`                                                                                |
| `<ng-flow>` (`ng-flow-box`, `gtk-flow-box`)            | Responsive Layout    | `GtkFlowBox`              | `Microsoft.UI.Xaml.Controls.ItemsRepeater`   | `NSCollectionView`    | `spacing` / `gap`, `row-spacing`, `column-spacing`, `min-children` / `min-per-row`, `max-children` / `max-per-row`, `homogeneous` |
| `<ng-scroll>` (`gtk-scrolled-window`)                  | Scroll Container     | `GtkScrolledWindow`       | `Microsoft.UI.Xaml.Controls.ScrollViewer`    | `NSScrollView`        | `min-height`                                                                                                                      |
| `<ng-separator>` (`gtk-separator`)                     | Layout Divider       | `GtkSeparator`            | `Microsoft.UI.Xaml.Controls.AppBarSeparator` | `NSBoxSeparator`      | `orientation` (`horizontal` \| `vertical`)                                                                                        |
| `<ng-label>` (`gtk-label`)                             | Typography           | `GtkLabel`                | `Microsoft.UI.Xaml.Controls.TextBlock`       | `NSTextField`         | text content / `{{ signal() }}`                                                                                                   |
| `<ng-image>` (`ng-picture`, `gtk-picture`)             | Media                | `GtkPicture`              | `Microsoft.UI.Xaml.Controls.Image`           | `NSImageView`         | `src`, `width`, `height`                                                                                                          |
| `<ng-level-bar>` (`gtk-level-bar`)                     | Meter / Indicator    | `GtkLevelBar`             | `Microsoft.UI.Xaml.Controls.ProgressBar`     | `NSLevelIndicator`    | `value`, `min`, `max`                                                                                                             |
| `<ng-progress-bar>` (`gtk-progress-bar`)               | Feedback             | `GtkProgressBar`          | `Microsoft.UI.Xaml.Controls.ProgressBar`     | `NSProgressIndicator` | `value` (`0.0..1.0`), `indeterminate`                                                                                             |
| `<ng-spinner>` (`gtk-spinner`)                         | Feedback             | `GtkSpinner`              | `Microsoft.UI.Xaml.Controls.ProgressRing`    | `NSProgressIndicator` | `active`                                                                                                                          |
| `<ng-button>` (`gtk-button`)                           | Action Control       | `GtkButton`               | `Microsoft.UI.Xaml.Controls.Button`          | `NSButton`            | label text, `(click)`                                                                                                             |
| `<ng-entry>` (`gtk-entry`)                             | Form Input           | `GtkEntry`                | `Microsoft.UI.Xaml.Controls.TextBox`         | `NSTextFieldEditable` | `value` / `text`, `placeholder`, `(input)` + `NativeTextValueAccessor`                                                            |
| `<ng-switch>` (`gtk-switch`)                           | Form Toggle          | `GtkSwitch`               | `Microsoft.UI.Xaml.Controls.ToggleSwitch`    | `NSSwitch`            | `active` / `checked` / `value`, `(change)` + `NativeBooleanValueAccessor`                                                         |
| `<ng-checkbox>` (`ng-check`, `gtk-check-button`)       | Form Toggle          | `GtkCheckButton`          | `Microsoft.UI.Xaml.Controls.CheckBox`        | `NSButtonCheckbox`    | `label`, `active` / `checked` / `value`, `(change)` + `NativeBooleanValueAccessor`                                                |
| `<ng-slider>` (`ng-scale`, `gtk-scale`)                | Form Range           | `GtkScale`                | `Microsoft.UI.Xaml.Controls.Slider`          | `NSSlider`            | `value`, `min`, `max`, `(valueChange)` + `NativeNumberValueAccessor`                                                              |
| `<ng-select>` (`ng-dropdown`, `gtk-drop-down`)         | Form Select          | `GtkDropDown`             | `Microsoft.UI.Xaml.Controls.ComboBox`        | `NSPopUpButton`       | `options` (CSV), `selected` / `value`, `(valueChange)` + `NativeNumberValueAccessor`                                              |
| `<ng-anchor>` (`#comment`)                             | Virtual Control Flow | `GtkAnchorComment`        | `Microsoft.UI.Xaml.AnchorComment`            | `NSAnchorComment`     | `@if`, `@for`, `@switch` anchor                                                                                                   |

> **Common Widget Attributes:** `visible`, `hexpand`, `vexpand`, `halign`, `valign`, `margin`, `margin-top`, `margin-bottom`, `margin-start`, `margin-end` (plus `sensitive` emitted by `@ng-native/forms` `setDisabledState`).

### Pending Primitives (Phase 8 Roadmap)

- **Priority Containers:**
  - `<ng-paned>` (`GtkPaned`): Resizable two-pane split container (`orientation`, `position`, start/end child slots).
  - `<ng-stack>` (`GtkStack` / `AdwNavigationView`): Multi-page view container (`visible-child`, `transition-type`, router integration).
  - `<ng-expander>` (`GtkExpander`): Collapsible disclosure / accordion container (`label`, `expanded`, `notify::expanded`).
- **Secondary Form & Adaptive Controls:**
  - `<ng-text-view>` (`GtkTextView`), `<ng-password-entry>` (`GtkPasswordEntry`), `<ng-spin-button>` (`GtkSpinButton`), `<ng-search-entry>` (`GtkSearchEntry`), `<ng-clamp>` (`AdwClamp`).

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
│   │   ├── mod.rs                # Desktop OS services (Theme, Clipboard, Notifications, Dialogs, HTTP)
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
│   │   ├── gtk4_styling.rs       # GTK4 CSS stylesheet builder & widget attribute dispatcher
│   │   ├── gtk4_sys.rs           # Isolated C FFI dynamic loader for libgtk-4.so.1
│   │   ├── gtk4_widgets.rs       # Safe GTK4 widget, layout & interactive control wrappers
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
│   ├── platform-native/          # @ng-native/platform custom Angular 22 Renderer2 package
│   ├── forms-native/             # @ng-native/forms ControlValueAccessor directives
│   ├── router-native/            # @ng-native/router DOM-free NativePlatformLocation & provideNativeRouter()
│   ├── http-native/              # @ng-native/http NativeHttpBackend & provideNativeHttpClient()
│   ├── desktop-native/           # @ng-native/desktop NativeDesktopService (theme Signal, clipboard, dialogs)
│   └── builders-native/          # @ng-native/builders Angular 22 Architect builder & packager orchestrator
├── examples/
│   └── counter-app/              # Official Angular 22 showcase application workspace
│       ├── ast-metadata-extractor.mjs # Rust oxc-parser TypeScript 6.0 AST extractor
│       ├── build-bundle.mjs      # AST bundle compiler & IPC reload notifier
│       ├── dev-server.mjs        # Live Hot-Reload dev orchestrator
│       ├── hmr-client.mjs        # Node.js Unix socket IPC client for instant HMR
│       ├── icon-compiler.mjs     # Multi-DPI PNG taskbar icon generator
│       ├── style-compiler.mjs    # PostCSS AST parser for native CSS styling
│       ├── template-compiler.mjs # @angular/compiler AST template parser
│       └── src/app/              # Angular 22 Component (app.ts, app.html, app.css)
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

This runs the official Angular 22 compiler (`ng build`) followed by the AST-driven native compiler (`examples/counter-app/build-bundle.mjs`), generating:

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

| Command                                     | Description                                                          |
| :------------------------------------------ | :------------------------------------------------------------------- |
| `pnpm run dev`                              | Watch Angular files & hot-reload native GTK4 window via Unix socket  |
| `pnpm run check`                            | Run full verification gate (OxFmt + OxLint + ESLint + Clippy + Test) |
| `pnpm run package:linux`                    | Build bundle and stage Linux `.deb`, AppImage, and Flatpak packages  |
| `pnpm run format:all`                       | Format both TypeScript/Angular (`oxfmt`) and Rust (`cargo fmt`)      |
| `pnpm run format`                           | Format TypeScript, HTML, CSS, JSON, and Markdown via `oxfmt`         |
| `pnpm run format:check`                     | Verify formatting with `oxfmt`                                       |
| `pnpm run lint`                             | Lint TypeScript & Angular templates via `oxlint` + `angular-eslint`  |
| `pnpm run test`                             | Run all 27 Vitest specs for packages and showcase app                |
| `cargo fmt --all -- --check`                | Verify Rust formatting                                               |
| `cargo clippy --all-targets -- -D warnings` | Run strict Rust Clippy lints (pedantic + nursery, zero warnings)     |
| `cargo test`                                | Execute all 25 unit tests + 14 E2E integration tests in Rust         |

---

## 🗺️ Roadmap & Changelog

- See [**ROADMAP.md**](./ROADMAP.md) for the complete technical analysis and phased milestones to reach **100% Angular ecosystem compatibility** on Linux, Windows, and macOS.
- See [**CHANGELOG.md**](./CHANGELOG.md) for release notes and detailed version history.
